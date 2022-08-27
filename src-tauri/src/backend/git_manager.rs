use std::collections::HashMap;
use std::path::PathBuf;
use directories::BaseDirs;
use git2::{AutotagOption, BranchType, Cred, Diff, DiffOptions, FetchOptions, FetchPrune, Oid, PushOptions, Reference, RemoteCallbacks, Repository, Sort};
use rfd::FileDialog;
use super::config_manager;

pub struct GitManager {
    repo: Option<Repository>,
}

impl GitManager {
    pub const fn new() -> Self {
        Self {
            repo: None,
        }
    }

    pub fn get_utf8_string<'a, 'b>(value: Option<&'a str>, str_name_type: &'b str) -> Result<&'a str, Box<dyn std::error::Error>> {
        match value {
            Some(n) => Ok(n),
            None => Err(format!("{} uses invalid utf-8!", str_name_type).into()),
        }
    }

    pub fn get_repo(&self) -> Result<&Repository, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        match repo_temp_opt {
            Some(repo) => Ok(repo),
            None => Err("No repo loaded to perform operation on.".into()),
        }
    }

    fn get_directory(&self) -> Option<PathBuf> {
        let bd_opt = BaseDirs::new();
        match bd_opt {
            Some(bd) => FileDialog::new().set_directory(bd.home_dir()).pick_folder(),
            None => FileDialog::new().set_directory(PathBuf::from("/")).pick_folder(),
        }
    }

    pub fn init_repo(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        match self.get_directory() {
            Some(path_buffer) => {
                self.repo = Some(Repository::init(path_buffer.as_path())?);
                Ok(true)
            },
            None => Ok(false),
        }
    }

    pub fn open_repo(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        match self.get_directory() {
            Some(path_buffer) => {
                self.repo = Some(Repository::open(path_buffer.as_path())?);
                Ok(true)
            },
            None => Ok(false),
        }
    }

    pub fn git_revwalk(&self) -> Result<Vec<Oid>, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let mut revwalk = repo.revwalk()?;
        let mut oid_vec: Vec<Oid> = vec![];
        for branch_result in repo.branches(None)? {
            let (branch, _) = branch_result?;
            match branch.get().target() {
                Some(oid) => oid_vec.push(oid),
                None => (),
            };
        };
        // Sort Oids by date first
        oid_vec.sort_by(|a, b| {
            repo.find_commit(*b).unwrap().time().seconds().partial_cmp(&repo.find_commit(*a).unwrap().time().seconds()).unwrap()
        });
        for oid in oid_vec {
            revwalk.push(oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;

        let preferences = config_manager::get_preferences()?;
        let limit_commits = preferences.get_limit_commits();
        let commit_count = preferences.get_commit_count();

        let mut oid_list: Vec<Oid> = vec![];
        for (i, commit_oid_result) in revwalk.enumerate() {
            if limit_commits && i >= commit_count {
                break;
            }
            oid_list.push(commit_oid_result?);
        }
        Ok(oid_list)
    }

    pub fn get_ref_from_name(&self, ref_full_name: &str) -> Result<Reference, Box<dyn std::error::Error>> {
        Ok(self.get_repo()?.find_reference(ref_full_name)?)
    }

    pub fn git_checkout(&self, local_ref: &Reference) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;
        let commit = match local_ref.target() {
            Some(oid) => repo.find_commit(oid)?,
            None => return Err("Trying to check out branch that has no target commit.".into()),
        };
        let tree = commit.tree()?;

        repo.checkout_tree(tree.as_object(), None)?;
        repo.set_head(local_full_name)?;
        Ok(())
    }

    pub fn git_checkout_remote(&self, json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let json_data: HashMap<String, String> = serde_json::from_str(json_string)?;
        let remote_branch_shortname = match json_data.get("branch_name") {
            Some(n) => n,
            None => return Err("JSON Data is missing branch_name attribute.".into()),
        };
        let remote_branch_full_name = match json_data.get("full_branch_name") {
            Some(n) => n,
            None => return Err("JSON Data is missing full_branch_name attribute.".into()),
        };

        // Look for a local branch that already exists for the specified remote branch. If one exists,
        // check it out instead.
        for local_b_result in repo.branches(Some(BranchType::Local))? {
            let (local_b, _) = local_b_result?;
            let local_upstream = match local_b.upstream() {
                Ok(b) => b,
                Err(_) => {
                    continue;
                },
            };
            let local_remote_full_name = GitManager::get_utf8_string(local_upstream.get().name(), "Branch Name")?;
            if local_remote_full_name == remote_branch_full_name {
                return self.git_checkout(local_b.get());
            }
        }

        // If there's no local branch, create a new one with the name used by the remote branch.
        let remote_branch_name_parts: Vec<&str> = remote_branch_shortname.split("/").collect();
        let mut local_branch_shortname = String::new();
        for i in 1..remote_branch_name_parts.len() {
            local_branch_shortname.push_str(remote_branch_name_parts[i]);
            if i < remote_branch_name_parts.len() - 1 {
                local_branch_shortname.push('/');
            }
        }
        let remote_branch = repo.find_branch(remote_branch_shortname, BranchType::Remote)?;
        let commit = match remote_branch.get().target() {
            Some(oid) => repo.find_commit(oid)?,
            None => return Err("Selected remote branch isn't targeting a commit, can't checkout!".into()),
        };
        let mut local_branch = repo.branch(&*local_branch_shortname, &commit, false)?;
        local_branch.set_upstream(Some(remote_branch_shortname))?;

        self.git_checkout(local_branch.get())
    }

    pub fn git_fetch(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let remote_string_array = repo.remotes()?;
        let empty_refspecs: &[String] = &[];
        for remote_string_opt in remote_string_array.iter() {
            let remote_string = GitManager::get_utf8_string(remote_string_opt, "Remote Name")?;
            let mut remote = repo.find_remote(remote_string)?;
            let mut fetch_options = FetchOptions::new();
            fetch_options.download_tags(AutotagOption::All);
            fetch_options.prune(FetchPrune::On);
            fetch_options.remote_callbacks(self.get_remote_callbacks());
            remote.fetch(empty_refspecs, Some(&mut fetch_options), None)?;
        }
        Ok(())
    }

    pub fn get_unstaged_changes(&self) -> Result<Diff, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        diff_options.recurse_untracked_dirs(true);

        let diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;
        Ok(diff)
    }

    pub fn get_staged_changes(&self) -> Result<Diff, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let head_ref = repo.head()?;
        let commit = match head_ref.target() {
            Some(oid) => Some(repo.find_commit(oid)?),
            None => None,
        };
        let tree = match commit {
            Some(c) => Some(c.tree()?),
            None => None,
        };

        let diff = repo.diff_tree_to_index(tree.as_ref(), None, None)?;

        Ok(diff)
    }

    pub fn git_pull(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        // Fetch first to make sure everything's up to date.
        self.git_fetch()?;

        let mut local_ref = repo.head()?;
        let local_shorthand = GitManager::get_utf8_string(local_ref.shorthand(), "Branch Name")?;
        let local_branch = repo.find_branch(local_shorthand, BranchType::Local)?;

        let remote_branch = local_branch.upstream()?;
        let remote_ref = remote_branch.get();
        let remote_target = match remote_ref.target() {
            Some(oid) => oid,
            None => return Err("Remote branch is not targeting a commit, cannot pull.".into()),
        };
        let remote_ac = repo.find_annotated_commit(remote_target)?;

        let (ma, mp) = repo.merge_analysis(&[&remote_ac])?;

        if ma.is_none() {
            return Err("Merge analysis indicates no merge is possible. If you're reading this, your repository may be corrupted.".into());
        } else if ma.is_unborn() {
            return Err("The HEAD of the current repository is “unborn” and does not point to a valid commit. No pull can be performed, but the caller may wish to simply set HEAD to the target commit(s).".into());
        } else if ma.is_up_to_date() {
            return Ok(());
        } else if ma.is_fast_forward() && !mp.is_no_fast_forward() {
            println!("Performing fast forward merge for pull!");
            let commit = match remote_ref.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => return Err("Trying to check out branch that has no target commit.".into()),
            };
            let tree = commit.tree()?;
            repo.checkout_tree(tree.as_object(), None)?;
            local_ref.set_target(remote_target, "oxidized_git pull: setting new target for local ref")?;
            return Ok(());
        } else if ma.is_normal() && !mp.is_fastforward_only() {
            println!("Performing rebase for pull!");
            let mut rebase = repo.rebase(None, None, Some(&remote_ac), None)?;
            let mut has_conflicts = false;
            for step in rebase.by_ref() {
                step?;
                let diff = repo.diff_index_to_workdir(None, None)?;
                if diff.stats()?.files_changed() > 0 {
                    has_conflicts = true;
                    break;
                }
            }
            if has_conflicts {
                rebase.abort()?;
                return Err("Pull by rebase aborted because changes on local branch differ from remote branch!".into());
            }
            rebase.finish(None)?;
            return Ok(());
        } else if (ma.is_fast_forward() && mp.is_no_fast_forward()) || (ma.is_normal() && mp.is_fastforward_only()) {
            return Err("It looks like a pull may be possible, but your MergePreference(s) are preventing it. If you have --no-ff AND/OR --ff-only enabled, consider disabling one or both.".into());
        }
        Err("Merge analysis failed to make any determination on how to proceed with the pull. If you're reading this, your repository may be corrupted.".into())
    }

    pub fn git_push(&self, push_options_json: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let push_options: HashMap<String, String> = serde_json::from_str(push_options_json)?;
        let is_force = match push_options.get("isForcePush") {
            Some(s) => s == "true",
            None => return Err("isForcePush not included in payload from front-end.".into()),
        };
        let remote_name_from_frontend = match push_options.get("selectedRemote") {
            Some(s) => s.as_str(),
            None => return Err("selectedRemote not included in payload from front-end.".into()),
        };

        let local_ref = repo.head()?;
        let local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;

        let mut is_creating_new_remote_branch = false;
        let mut remote = match repo.branch_upstream_remote(local_full_name) {
            Ok(b) => {
                let remote_name = GitManager::get_utf8_string(b.as_str(), "Remote Name")?;
                repo.find_remote(remote_name)?
            },
            Err(_e) => {
                is_creating_new_remote_branch = true;
                repo.find_remote(remote_name_from_frontend)?
            },
        };

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.get_remote_callbacks());

        let mut sb = String::from(local_full_name);
        if is_force {
            sb.insert(0, '+');
        }

        remote.push(&[sb.as_str()], Some(&mut push_options))?;

        if is_creating_new_remote_branch {
            let local_branch_shorthand = GitManager::get_utf8_string(local_ref.shorthand(), "Branch Name")?;
            let new_remote_branch_shorthand = format!("{remote_name_from_frontend}/{local_branch_shorthand}");
            let mut local_branch = repo.find_branch(local_branch_shorthand, BranchType::Local)?;
            local_branch.set_upstream(Some(&*new_remote_branch_shorthand))?;
        }

        Ok(())
    }

    #[allow(unused_unsafe)]
    fn get_remote_callbacks(&self) -> RemoteCallbacks {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            let username;
            let pass;
            unsafe {
                username = match keytar::get_password("oxidized_git", "username") {
                    Ok(p) => p,
                    Err(_) => return Err(git2::Error::from_str("Error finding username in keychain!")),
                };
                pass = match keytar::get_password("oxidized_git", "password") {
                    Ok(p) => p,
                    Err(_) => return Err(git2::Error::from_str("Error finding password in keychain!")),
                };
            }
            if username.success && pass.success {
                Cred::userpass_plaintext(&*username.password, &*pass.password)
            } else {
                Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials"))
            }
        });
        callbacks.push_update_reference(|_ref_name, status_msg| {
            match status_msg {
                Some(m) => Err(git2::Error::from_str(&*format!("Error(s) during push: {}", m))),
                None => Ok(()),
            }
        });
        callbacks
    }

    #[allow(unused_unsafe)]
    pub fn set_credentials(&self, credentials_json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let credentials_json: HashMap<String, String> = serde_json::from_str(credentials_json_string)?;
        let username = match credentials_json.get("username") {
            Some(u) => u,
            None => return Err("No username supplied".into()),
        };
        let password = match credentials_json.get("password") {
            Some(p) => p,
            None => return Err("No password supplied".into()),
        };

        unsafe {
            keytar::set_password("oxidized_git", "username", username)?;
            keytar::set_password("oxidized_git", "password", password)?;
        }

        Ok(())
    }
}

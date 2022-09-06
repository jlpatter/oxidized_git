use std::collections::HashMap;
use std::path::PathBuf;
use std::str;
use directories::BaseDirs;
use git2::{AutotagOption, BranchType, Cred, Diff, DiffFindOptions, DiffLine, DiffOptions, FetchOptions, FetchPrune, Oid, Patch, PushOptions, Reference, RemoteCallbacks, Repository, Sort};
use rfd::FileDialog;
use serde::Serialize;
use crate::backend::parseable_info::ParseableDiffDelta;
use super::config_manager;

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
    }
    if s.ends_with('\r') {
        s.pop();
    }
}

#[derive(Clone, Serialize)]
pub struct FileLineInfo {
    old_lineno: Option<u32>,
    new_lineno: Option<u32>,
    file_type: String,
    content: String,
    origin: char,
}

impl FileLineInfo {
    pub fn from_diff_line(diff_line: DiffLine, file_type: &String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut content_string = String::from(str::from_utf8(diff_line.content())?);
        trim_newline(&mut content_string);
        content_string = html_escape::encode_text(&content_string).parse()?;
        let new_info = Self {
            old_lineno: diff_line.old_lineno(),
            new_lineno: diff_line.new_lineno(),
            file_type: file_type.clone(),
            content: content_string,
            origin: diff_line.origin(),
        };
        Ok(new_info)
    }
}

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

    pub fn has_open_repo(&self) -> bool {
        match self.get_repo() {
            Ok(_) => true,
            Err(_) => false,
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
        let remote_branch_shortname = match json_data.get("branch_shorthand") {
            Some(n) => n,
            None => return Err("JSON Data is missing branch_shorthand attribute.".into()),
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

    pub fn git_stage(&self, json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_string)?;

        let mut index = repo.index()?;
        if diff_delta.get_status() == 2 {  // If file is deleted
            index.remove_path(diff_delta.get_path().as_ref())?;
        } else {
            index.add_path(diff_delta.get_path().as_ref())?;
        }
        index.write()?;

        Ok(())
    }

    pub fn git_unstage(&self, json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_string)?;

        let mut index = repo.index()?;
        let status = diff_delta.get_status();
        if status == 2 || status == 3 {  // If file is deleted or modified
            let head_commit = match repo.head()?.target() {
                Some(oid) => {
                    repo.find_commit(oid)?
                },
                None => return Err("Head has no target commit".into()),
            };
            repo.reset_default(Some(head_commit.as_object()), [diff_delta.get_path()])?;
        } else {
            index.remove_path(diff_delta.get_path().as_ref())?;
        }
        index.write()?;

        Ok(())
    }

    fn set_diff_find_similar(diff: &mut Diff) -> Result<(), Box<dyn std::error::Error>> {
        let mut opts = DiffFindOptions::new();
        opts.renames(true);
        opts.copies(true);

        diff.find_similar(Some(&mut opts))?;
        Ok(())
    }

    pub fn get_unstaged_changes(&self) -> Result<Diff, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        diff_options.recurse_untracked_dirs(true);
        diff_options.show_untracked_content(true);

        let mut diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;
        GitManager::set_diff_find_similar(&mut diff)?;

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

        let mut diff = repo.diff_tree_to_index(tree.as_ref(), None, None)?;
        GitManager::set_diff_find_similar(&mut diff)?;

        Ok(diff)
    }

    pub fn get_file_diff(&self, json_str: &str) -> Result<Vec<FileLineInfo>, Box<dyn std::error::Error>> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let file_path = match json_hm.get("file_path") {
            Some(s) => s,
            None => return Err("file_path not returned from front-end payload.".into()),
        };
        let change_type = match json_hm.get("change_type") {
            Some(s) => s,
            None => return Err("change_type not returned from front-end payload.".into()),
        };

        let diff;
        if change_type == "unstaged" {
            diff = self.get_unstaged_changes()?;
        } else if change_type == "staged" {
            diff = self.get_staged_changes()?;
        } else {
            return Err("change_type not a valid type. Needs to be 'staged' or 'unstaged'".into());
        }

        let file_index_opt = diff.deltas().position(|dd| {
            match dd.new_file().path() {
                Some(p) => {
                    match p.to_str() {
                        Some(s) => file_path.as_str() == s,
                        None => false,
                    }
                },
                None => false,
            }
        });
        let file_index = match file_index_opt {
            Some(i) => i,
            None => return Err("Selected file not found. This shouldn't happen since this uses the same methods that are used to generate the file list.".into()),
        };

        let patch_opt = Patch::from_diff(&diff, file_index)?;
        let mut file_lines = vec![];
        let file_type = String::from(file_path.split(".").last().unwrap_or(""));
        match patch_opt {
            Some(patch) => {
                for i in 0..patch.num_hunks() {
                    let line_count = patch.num_lines_in_hunk(i)?;
                    for j in 0..line_count {
                        let line = patch.line_in_hunk(i, j)?;
                        file_lines.push(FileLineInfo::from_diff_line(line, &file_type)?);
                    }
                }
            },
            None => return Err("Patch not found in diff.".into()),
        }
        Ok(file_lines)
    }

    pub fn git_commit(&self, json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        // TODO: Add way to set signature in git config
        let signature = repo.signature()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_string)?;
        let summary = match json_hm.get("summaryText") {
            Some(s) => s,
            None => return Err("Front-end payload did not include summaryText".into()),
        };
        let message = match json_hm.get("messageText") {
            Some(s) => s,
            None => return Err("Front-end payload did not include messageText".into()),
        };

        let mut full_message = summary.clone();
        if message != "" {
            full_message.push_str("\n\n");
            full_message.push_str(message);
        }

        let mut parents = vec![];
        let commit;
        if let Some(oid) = repo.head()?.target() {
            commit = repo.find_commit(oid)?;
            parents.push(&commit);
        }

        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        index.write()?;
        let tree = repo.find_tree(tree_oid)?;

        repo.commit(Some("HEAD"), &signature, &signature, &*full_message, &tree, parents.as_slice())?;
        Ok(())
    }

    pub fn git_delete_local_branch(&self, branch_shorthand: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let mut branch = repo.find_branch(branch_shorthand, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    pub fn git_delete_remote_branch(&self, branch_shorthand: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.get_remote_callbacks());

        let mut sb = String::from(":refs/heads/");
        let first_slash_index = match branch_shorthand.find("/") {
            Some(i) => i,
            None => return Err("Remote Branch doesn't seem to have a remote in its name?".into()),
        };
        let mut remote = repo.find_remote(&branch_shorthand[0..first_slash_index])?;
        sb.push_str(&branch_shorthand[(first_slash_index + 1)..]);
        remote.push(&[sb.as_str()], Some(&mut push_options))?;
        Ok(())
    }

    pub fn git_delete_tag(&self, tag_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        repo.tag_delete(tag_name)?;
        Ok(())
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

    pub fn git_push(&self, push_options_json_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;

        let is_force;
        let remote_name_from_frontend_opt;
        let push_options: HashMap<String, String>;
        if let Some(push_options_json) = push_options_json_opt {
            push_options = serde_json::from_str(push_options_json)?;
            is_force = match push_options.get("isForcePush") {
                Some(s) => s == "true",
                None => return Err("isForcePush not included in payload from front-end.".into()),
            };
            remote_name_from_frontend_opt = match push_options.get("selectedRemote") {
                Some(s) => Some(s.as_str()),
                None => return Err("selectedRemote not included in payload from front-end.".into()),
            };
        } else {
            is_force = false;
            remote_name_from_frontend_opt = None;
        }

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
                match remote_name_from_frontend_opt {
                    Some(rn) => repo.find_remote(rn)?,
                    None => return Err("Attempted to push with no upstream branch and no specified remote.".into()),
                }
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
            let remote_name = GitManager::get_utf8_string(remote.name(), "Remote Name")?;
            let new_remote_branch_shorthand = format!("{remote_name}/{local_branch_shorthand}");
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

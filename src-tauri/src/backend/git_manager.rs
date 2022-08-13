use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::path::PathBuf;
use git2::{AutotagOption, BranchType, Cred, Diff, FetchOptions, Oid, PushOptions, Reference, RemoteCallbacks, Repository, Sort};
use home::home_dir;
use rfd::FileDialog;
use serde::{Serialize, Serializer};

pub enum CommitInfoValue {
    SomeString(String),
    SomeStringVec(Vec<String>),
    SomeHashMapVec(Vec<HashMap<String, String>>),
    SomeInt(u64),
}

impl Serialize for CommitInfoValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            CommitInfoValue::SomeString(st) => st.serialize(serializer),
            CommitInfoValue::SomeStringVec(v) => v.serialize(serializer),
            CommitInfoValue::SomeHashMapVec(v) => v.serialize(serializer),
            CommitInfoValue::SomeInt(i) => i.serialize(serializer),
        }
    }
}

impl Clone for CommitInfoValue {
    fn clone(&self) -> Self {
        match &self {
            CommitInfoValue::SomeString(s) => CommitInfoValue::SomeString(s.clone()),
            CommitInfoValue::SomeStringVec(v) => CommitInfoValue::SomeStringVec(v.clone()),
            CommitInfoValue::SomeHashMapVec(v) => CommitInfoValue::SomeHashMapVec(v.clone()),
            CommitInfoValue::SomeInt(i) => CommitInfoValue::SomeInt(i.clone()),
        }
    }
}

pub enum RepoInfoValue {
    SomeCommitInfo(Vec<HashMap<String, CommitInfoValue>>),
    SomeBranchInfo(Vec<HashMap<String, String>>),
}

impl Serialize for RepoInfoValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            RepoInfoValue::SomeCommitInfo(c) => c.serialize(serializer),
            RepoInfoValue::SomeBranchInfo(b) => b.serialize(serializer),
        }
    }
}

impl Clone for RepoInfoValue {
    fn clone(&self) -> Self {
        match &self {
            RepoInfoValue::SomeCommitInfo(c) => RepoInfoValue::SomeCommitInfo(c.clone()),
            RepoInfoValue::SomeBranchInfo(b) => RepoInfoValue::SomeBranchInfo(b.clone()),
        }
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

    fn get_utf8_string<'a, 'b>(value: Option<&'a str>, str_name_type: &'b str) -> Result<&'a str, Box<dyn std::error::Error>> {
        match value {
            Some(n) => Ok(n),
            None => Err(format!("{} uses invalid utf-8!", str_name_type).into()),
        }
    }

    fn get_directory(&self) -> Option<PathBuf> {
        let home;
        match home_dir() {
            Some(d) => home = d,
            None => home = PathBuf::from("/"),
        }
        FileDialog::new().set_directory(home).pick_folder()
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

    fn git_revwalk(&self) -> Result<Vec<Oid>, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get commit lines for.".into()),
        };
        let mut revwalk = repo_temp.revwalk()?;
        let mut oid_vec: Vec<Oid> = vec![];
        for branch_result in repo_temp.branches(None)? {
            let (branch, _) = branch_result?;
            match branch.get().target() {
                Some(oid) => oid_vec.push(oid),
                None => (),
            };
        };
        // Sort Oids by date first
        oid_vec.sort_by(|a, b| {
            repo_temp.find_commit(*b).unwrap().time().seconds().partial_cmp(&repo_temp.find_commit(*a).unwrap().time().seconds()).unwrap()
        });
        for oid in oid_vec {
            revwalk.push(oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;
        let mut oid_list: Vec<Oid> = vec![];
        for commit_oid_result in revwalk {
            oid_list.push(commit_oid_result?);
        }
        Ok(oid_list)
    }

    fn get_oid_refs(&self) -> Result<HashMap<String, Vec<HashMap<String, String>>>, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get repo info for.".into()),
        };

        // Get HashMap of Oids and their refs based on type (local, remote, or tag)
        let mut oid_refs: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();

        // Iterate over branches
        for branch_result in repo_temp.branches(None)? {
            let (branch, _) = branch_result?;
            let mut branch_string = String::new();
            if branch.is_head() {
                branch_string.push_str("* ");
            }

            let reference = branch.get();
            let ref_shorthand = GitManager::get_utf8_string(reference.shorthand(), "Ref Name")?;
            branch_string.push_str(ref_shorthand);
            match reference.target() {
                Some(oid) => {
                    let mut branch_info_hm: HashMap<String, String> = HashMap::new();
                    branch_info_hm.insert("branch_name".to_string(), branch_string);
                    if reference.is_remote() {
                        branch_info_hm.insert("branch_type".to_string(), "remote".to_string());
                    } else {
                        branch_info_hm.insert("branch_type".to_string(), "local".to_string());
                    }
                    match oid_refs.get_mut(&*oid.to_string()) {
                        Some(oid_ref_vec) => {
                            oid_ref_vec.push(branch_info_hm);
                        }
                        None => {
                            oid_refs.insert(oid.to_string(), vec![branch_info_hm]);
                        },
                    }
                },
                None => (),
            };
        }

        // Iterate over tags
        for reference_result in repo_temp.references()? {
            let reference = reference_result?;
            if reference.is_tag() {
                let ref_name = GitManager::get_utf8_string(reference.shorthand(), "Tag Name")?;

                match reference.target() {
                    Some(oid) => {
                        let mut tag_info_hm: HashMap<String, String> = HashMap::new();
                        tag_info_hm.insert("branch_name".to_string(), ref_name.to_string());
                        tag_info_hm.insert("branch_type".to_string(), "tag".to_string());
                        match oid_refs.get_mut(&*oid.to_string()) {
                            Some(oid_ref_vec) => {
                                oid_ref_vec.push(tag_info_hm);
                            }
                            None => {
                                oid_refs.insert(oid.to_string(), vec![tag_info_hm]);
                            },
                        };
                    },
                    None => (),
                }
            }
        }
        Ok(oid_refs)
    }

    fn get_commit_info_list(&self, oid_list: Vec<Oid>) -> Result<Vec<HashMap<String, CommitInfoValue>>, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get commit info for.".into()),
        };

        let mut commit_list: Vec<HashMap<String, CommitInfoValue>> = vec![];
        let oid_refs_hm = self.get_oid_refs()?;

        let mut children_oids: HashMap<String, Vec<String>> = HashMap::new();
        for (i, oid) in oid_list.iter().enumerate() {
            let mut commit_info: HashMap<String, CommitInfoValue> = HashMap::new();
            commit_info.insert("oid".into(), CommitInfoValue::SomeString(oid.to_string()));
            commit_info.insert("x".into(), CommitInfoValue::SomeInt(0u64));
            commit_info.insert("y".into(), CommitInfoValue::SomeInt(i as u64));

            let commit = repo_temp.find_commit(*oid)?;

            // Get commit summary
            let commit_summary = GitManager::get_utf8_string(commit.summary(), "Commit Summary")?;
            commit_info.insert("summary".into(), CommitInfoValue::SomeString(commit_summary.into()));

            // Get branches pointing to this commit
            match oid_refs_hm.get(&*oid.to_string()) {
                Some(ref_vec) => {
                    commit_info.insert("branches_and_tags".into(), CommitInfoValue::SomeHashMapVec(ref_vec.clone()));
                }
                None => {
                    commit_info.insert("branches_and_tags".into(), CommitInfoValue::SomeHashMapVec(vec![]));
                },
            };

            // Get parent Oids
            let mut parent_oids: Vec<String> = vec![];
            for parent in commit.parents() {
                parent_oids.push(parent.id().to_string());
                match children_oids.get_mut(&*parent.id().to_string()) {
                    Some(children_oid_vec) => children_oid_vec.push(oid.to_string()),
                    None => {
                        children_oids.insert(parent.id().to_string(), vec![oid.to_string()]);
                    },
                };
            }

            commit_info.insert("parent_oids".into(), CommitInfoValue::SomeStringVec(parent_oids));
            commit_info.insert("child_oids".into(), CommitInfoValue::SomeStringVec(vec![]));
            commit_list.push(commit_info);
        }

        // Gather the child commits after running through the commit graph once in order
        // to actually have populated entries.
        for commit_hm in commit_list.iter_mut() {
            let oid_string = match commit_hm.get("oid") {
                Some(oid) => {
                    match oid {
                        CommitInfoValue::SomeString(oid_string) => oid_string,
                        CommitInfoValue::SomeStringVec(_some_vector) => return Err("Oid was stored as a vector instead of a string.".into()),
                        CommitInfoValue::SomeHashMapVec(_some_hm) => return Err("Oid was stored as a hashmap instead of a string.".into()),
                        CommitInfoValue::SomeInt(_some_int) => return Err("Oid was stored as an int instead of a string.".into()),
                    }
                },
                None => return Err("Commit found with no oid, shouldn't be possible...".into()),
            };
            match children_oids.get(oid_string) {
                Some(v) => {
                    commit_hm.insert("child_oids".into(), CommitInfoValue::SomeStringVec(v.clone()));
                },
                None => (),
            };
        }

        Ok(commit_list)
    }

    fn get_branch_info_list(&self) -> Result<Vec<HashMap<String, String>>, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get branch info for.".into()),
        };

        let mut branch_info_list: Vec<HashMap<String, String>> = vec![];

        for reference_result in repo_temp.references()? {
            let reference = reference_result?;
            let mut branch_info: HashMap<String, String> = HashMap::new();

            // Get branch name
            let branch_shorthand = GitManager::get_utf8_string(reference.shorthand(), "Branch Name")?;
            branch_info.insert("branch_name".to_string(), branch_shorthand.to_string());

            // Get full branch name
            let full_branch_name = GitManager::get_utf8_string(reference.name(), "Branch Name")?;
            branch_info.insert("full_branch_name".to_string(), full_branch_name.to_string());

            // Get if branch is head
            branch_info.insert("is_head".to_string(), false.to_string());
            if reference.is_branch() {
                let local_branch = repo_temp.find_branch(branch_shorthand, BranchType::Local)?;
                if local_branch.is_head() {
                    branch_info.insert("is_head".to_string(), true.to_string());
                }
            }

            // Get branch type
            if reference.is_branch() {
                branch_info.insert("branch_type".to_string(), "local".to_string());
            } else if reference.is_remote() {
                branch_info.insert("branch_type".to_string(), "remote".to_string());
            } else if reference.is_tag() {
                branch_info.insert("branch_type".to_string(), "tag".to_string());
            }

            // Get ahead/behind counts
            branch_info.insert("ahead".to_string(), "0".to_string());
            branch_info.insert("behind".to_string(), "0".to_string());
            if reference.is_branch() {
                let local_branch = repo_temp.find_branch(branch_shorthand, BranchType::Local)?;
                // This throws an error when an upstream isn't found, which is why I'm not returning the error.
                match local_branch.upstream() {
                    Ok(remote_branch) => {
                        match local_branch.get().target() {
                            Some(local_oid) => {
                                match remote_branch.get().target() {
                                    Some(remote_oid) => {
                                        let (ahead, behind) = repo_temp.graph_ahead_behind(local_oid, remote_oid)?;
                                        branch_info.insert("ahead".to_string(), ahead.to_string());
                                        branch_info.insert("behind".to_string(), behind.to_string());
                                    },
                                    None => (),
                                };
                            },
                            None => (),
                        };
                    },
                    Err(_) => (),
                };
            }

            branch_info_list.push(branch_info);
        }

        Ok(branch_info_list)
    }

    pub fn get_parseable_repo_info(&self) -> Result<HashMap<String, RepoInfoValue>, Box<dyn std::error::Error>> {
        let mut repo_info: HashMap<String, RepoInfoValue> = HashMap::new();
        repo_info.insert("commit_info_list".to_string(), RepoInfoValue::SomeCommitInfo(self.get_commit_info_list(self.git_revwalk()?)?));
        repo_info.insert("branch_info_list".to_string(), RepoInfoValue::SomeBranchInfo(self.get_branch_info_list()?));
        Ok(repo_info)
    }

    pub fn get_ref_from_name(&self, ref_full_name: &str) -> Result<Reference, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get branch name from.".into()),
        };

        Ok(repo_temp.find_reference(ref_full_name)?)
    }

    pub fn git_checkout(&self, local_ref: &Reference) -> Result<(), Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to checkout for.".into()),
        };

        let local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;
        let commit = match local_ref.target() {
            Some(oid) => repo_temp.find_commit(oid)?,
            None => return Err("Trying to check out branch that has no target commit.".into()),
        };
        let tree = commit.tree()?;

        repo_temp.checkout_tree(tree.as_object(), None)?;
        repo_temp.set_head(local_full_name)?;
        Ok(())
    }

    pub fn git_checkout_remote(&self, json_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to checkout for.".into()),
        };

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
        for local_b_result in repo_temp.branches(Some(BranchType::Local))? {
            let (local_b, _) = local_b_result?;
            let local_upstream = local_b.upstream()?;
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
        let remote_branch = repo_temp.find_branch(remote_branch_shortname, BranchType::Remote)?;
        let commit = match remote_branch.get().target() {
            Some(oid) => repo_temp.find_commit(oid)?,
            None => return Err("Selected remote branch isn't targeting a commit, can't checkout!".into()),
        };
        let mut local_branch = repo_temp.branch(&*local_branch_shortname, &commit, false)?;
        local_branch.set_upstream(Some(remote_branch_shortname))?;

        self.git_checkout(local_branch.get())
    }

    pub fn git_fetch(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to fetch for.".into()),
        };
        let remote_string_array = repo_temp.remotes()?;
        let empty_refspecs: &[String] = &[];
        for remote_string_opt in remote_string_array.iter() {
            let remote_string = GitManager::get_utf8_string(remote_string_opt, "Remote Name")?;
            let mut remote = repo_temp.find_remote(remote_string)?;
            let mut fetch_options = FetchOptions::new();
            fetch_options.download_tags(AutotagOption::All);
            fetch_options.remote_callbacks(self.get_remote_callbacks());
            remote.fetch(empty_refspecs, Some(fetch_options.borrow_mut()), None)?;
        }
        Ok(())
    }

    fn get_staged_changes(&self) -> Result<Diff, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get staged changes for.".into()),
        };

        let head_ref = repo_temp.head()?;
        let commit = match head_ref.target() {
            Some(oid) => Some(repo_temp.find_commit(oid)?),
            None => None,
        };
        let tree = match commit {
            Some(c) => Some(c.tree()?),
            None => None,
        };

        let diff = repo_temp.diff_tree_to_index(tree.as_ref(), None, None)?;

        Ok(diff)
    }

    pub fn git_pull(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to pull for.".into()),
        };

        // Fetch first to make sure everything's up to date.
        self.git_fetch()?;

        let mut local_ref = repo_temp.head()?;
        let local_shorthand = GitManager::get_utf8_string(local_ref.shorthand(), "Branch Name")?;
        let local_branch = repo_temp.find_branch(local_shorthand, BranchType::Local)?;

        let remote_branch = local_branch.upstream()?;
        let remote_ref = remote_branch.get();
        let remote_target = match remote_ref.target() {
            Some(oid) => oid,
            None => return Err("Remote branch is not targeting a commit, cannot pull.".into()),
        };
        let remote_ac = repo_temp.find_annotated_commit(remote_target)?;

        let (ma, mp) = repo_temp.merge_analysis(&[&remote_ac])?;

        if ma.is_none() {
            return Err("Merge analysis indicates no merge is possible. If you're reading this, your repository may be corrupted.".into());
        } else if ma.is_unborn() {
            return Err("The HEAD of the current repository is “unborn” and does not point to a valid commit. No pull can be performed, but the caller may wish to simply set HEAD to the target commit(s).".into());
        } else if ma.is_up_to_date() {
            return Ok(());
        } else if ma.is_fast_forward() && !mp.is_no_fast_forward() {
            println!("Performing fast forward merge for pull!");
            let commit = match remote_ref.target() {
                Some(oid) => repo_temp.find_commit(oid)?,
                None => return Err("Trying to check out branch that has no target commit.".into()),
            };
            let tree = commit.tree()?;
            repo_temp.checkout_tree(tree.as_object(), None)?;
            local_ref.set_target(remote_target, "oxidized_git pull: setting new target for local ref")?;
            return Ok(());
        } else if ma.is_normal() && !mp.is_fastforward_only() {
            println!("Performing rebase for pull!");
            let mut rebase = repo_temp.rebase(None, None, Some(&remote_ac), None)?;
            let mut has_conflicts = false;
            for step in rebase.by_ref() {
                step?;
                let diff = repo_temp.diff_index_to_workdir(None, None)?;
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

    fn push(&self, is_force: bool) -> Result<(), Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to pull for.".into()),
        };

        let local_ref = repo_temp.head()?;
        let mut local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;
        let remote_name_buf = repo_temp.branch_upstream_remote(local_full_name)?;
        let remote_name = GitManager::get_utf8_string(remote_name_buf.as_str(), "Remote Name")?;

        let mut remote = repo_temp.find_remote(remote_name)?;
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.get_remote_callbacks());

        let mut sb = String::from(local_full_name);
        if is_force {
            sb.insert_str(0, "+");
            local_full_name = sb.as_str();
        }

        remote.push(&[local_full_name], Some(push_options.borrow_mut()))?;
        Ok(())
    }

    pub fn git_push(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.push(false)
    }

    pub fn git_force_push(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.push(true)
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

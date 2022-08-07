use std::collections::HashMap;
use std::path::PathBuf;
use git2::{Oid, Repository, Sort};
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

pub struct GitManager {
    repo: Option<Repository>,
}

impl GitManager {
    pub const fn new() -> Self {
        Self {
            repo: None,
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
        for branch_result in repo_temp.branches(None)? {
            let (branch, _) = branch_result?;
            let reference = branch.get();
            match reference.target() {
                Some(oid) => revwalk.push(oid)?,
                None => (),
            };
        };
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
            let ref_name = match reference.shorthand() {
                Some(n) => n,
                None => return Err("Ref has name that's not utf-8 valid.".into()),
            };
            branch_string.push_str(ref_name);
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
                let ref_name = match reference.shorthand() {
                    Some(n) => n,
                    None => return Err("Tag has name that's not utf-8 valid.".into()),
                };

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
            None => return Err("No repo to get repo info for.".into()),
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
            match commit.summary() {
                Some(s) => commit_info.insert("summary".into(), CommitInfoValue::SomeString(s.into())),
                None => return Err("Commit summary didn't use proper utf-8!".into()),
            };

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

    pub fn get_parseable_repo_info(&self) -> Result<Vec<HashMap<String, CommitInfoValue>>, Box<dyn std::error::Error>> {
        let repo_info = self.get_commit_info_list(self.git_revwalk()?)?;
        Ok(repo_info)
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
            let remote_string = match remote_string_opt {
                Some(remote_string) => remote_string,
                None => return Err("There is a remote name that uses invalid utf-8!".into()),
            };
            let mut remote = repo_temp.find_remote(remote_string)?;
            remote.fetch(empty_refspecs, None, None)?;
        }
        println!("Fetch successful!");
        Ok(())
    }
}

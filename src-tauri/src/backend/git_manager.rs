use std::collections::HashMap;
use std::path::PathBuf;
use git2::{Oid, Repository, Sort};
use home::home_dir;
use rfd::FileDialog;
use serde::{Serialize, Serializer};

pub enum StringOrStringVec {
    SomeString(String),
    SomeStringVec(Vec<String>),
}

impl Serialize for StringOrStringVec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            StringOrStringVec::SomeString(st) => st.serialize(serializer),
            StringOrStringVec::SomeStringVec(v) => v.serialize(serializer),
        }
    }
}

impl Clone for StringOrStringVec {
    fn clone(&self) -> Self {
        match &self {
            StringOrStringVec::SomeString(s) => StringOrStringVec::SomeString(s.clone()),
            StringOrStringVec::SomeStringVec(v) => StringOrStringVec::SomeStringVec(v.clone()),
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

    fn get_commit_info_list(&self, oid_list: Vec<Oid>) -> Result<Vec<HashMap<String, StringOrStringVec>>, Box<dyn std::error::Error>> {
        let repo_temp_opt = &self.repo;
        let repo_temp = match repo_temp_opt {
            Some(repo) => repo,
            None => return Err("No repo to get repo info for.".into()),
        };

        let mut commit_list: Vec<HashMap<String, StringOrStringVec>> = vec![];

        let mut children_oids: HashMap<String, Vec<String>> = HashMap::new();
        for oid in oid_list {
            let mut commit_info: HashMap<String, StringOrStringVec> = HashMap::new();
            commit_info.insert("oid".into(), StringOrStringVec::SomeString(oid.to_string()));

            let mut parent_oids: Vec<String> = vec![];
            let commit = repo_temp.find_commit(oid)?;
            for parent in commit.parents() {
                parent_oids.push(parent.id().to_string());
                match children_oids.get_mut(&*parent.id().to_string()) {
                    Some(children_oid_vec) => children_oid_vec.push(oid.to_string()),
                    None => {
                        children_oids.insert(parent.id().to_string(), vec![oid.to_string()]);
                    },
                };
            }

            commit_info.insert("parent_oids".into(), StringOrStringVec::SomeStringVec(parent_oids));
            commit_info.insert("child_oids".into(), StringOrStringVec::SomeStringVec(vec![]));
            commit_list.push(commit_info);
        }

        // Gather the child commits after running through the commit graph once in order
        // to actually have populated entries.
        for commit_hm in commit_list.iter_mut() {
            let oid_string = match commit_hm.get("oid") {
                Some(oid) => {
                    match oid {
                        StringOrStringVec::SomeString(oid_string) => oid_string,
                        StringOrStringVec::SomeStringVec(_some_vector) => return Err("Oid was stored as a vector instead of a string.".into()),
                    }
                },
                None => return Err("Commit found with no oid, shouldn't be possible...".into()),
            };
            match children_oids.get(oid_string) {
                Some(v) => {
                    commit_hm.insert("child_oids".into(), StringOrStringVec::SomeStringVec(v.clone()));
                },
                None => (),
            };
        }

        Ok(commit_list)
    }

    pub fn get_parseable_repo_info(&self) -> Result<Vec<HashMap<String, StringOrStringVec>>, Box<dyn std::error::Error>> {
        let oid_list = self.git_revwalk()?;
        let repo_info = self.get_commit_info_list(oid_list)?;
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

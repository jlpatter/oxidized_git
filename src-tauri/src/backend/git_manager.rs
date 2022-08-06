use std::path::PathBuf;
use git2::{Oid, Repository, Sort};
use home::home_dir;
use rfd::FileDialog;

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

    pub fn get_all_commit_lines(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
        let mut message_list: Vec<String> = vec![];
        for oid in oid_list {
            match repo_temp.find_commit(oid)?.summary() {
                Some(s) => message_list.push(s.parse()?),
                None => return Err("There is a commit message that uses invalid utf-8!".into()),
            }
        }
        Ok(message_list)
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

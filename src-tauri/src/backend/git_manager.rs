use std::path::PathBuf;
use git2::{Oid, Repository, Sort};
use home::home_dir;
use rfd::FileDialog;

static mut REPO: Option<Repository> = None;

fn get_directory() -> Option<PathBuf> {
    let home;
    match home_dir() {
        Some(d) => home = d,
        None => home = PathBuf::from("/"),
    }
    FileDialog::new().set_directory(home).pick_folder()
}

pub fn init_repo() -> Result<(), String> {
    match get_directory() {
        Some(path_buffer) => {
            match Repository::init(path_buffer.as_path()) {
                Ok(repo) => {
                    unsafe {
                        REPO = Some(repo);
                    }
                    // TODO: Remove this println once the commit view is implemented.
                    println!("Repo initialized successfully!");
                    Ok(())
                },
                Err(e) => Err(e.message().into()),
            }
        },
        None => Ok(()),
    }
}

pub fn open_repo() -> Result<(), String> {
    match get_directory() {
        Some(path_buffer) => {
            match Repository::open(path_buffer.as_path()) {
                Ok(repo) => {
                    unsafe {
                        REPO = Some(repo);
                    }
                    // TODO: Remove this println once the commit view is implemented.
                    println!("Repo opened successfully!");
                    Ok(())
                },
                Err(e) => Err(e.message().into()),
            }
        },
        None => Ok(()),
    }
}

pub fn get_all_commit_lines() -> Result<Vec<String>, String> {
    let repo_temp_opt;
    unsafe {
        repo_temp_opt = &REPO;
    }
    let repo_temp = match repo_temp_opt {
        Some(repo) => repo,
        None => return Err("No repo to fetch for.".into()),
    };
    let branches = match repo_temp.branches(None) {
        Ok(b) => b,
        Err(e) => return Err(format!("Error getting list of branches from repo: {}", e)),
    };
    let mut revwalk = match repo_temp.revwalk() {
        Ok(r) => r,
        Err(e) => return Err(format!("Error getting revwalk object from repo: {}", e)),
    };
    for branch_result in branches {
        let (branch, _) = branch_result.unwrap();
        let reference = branch.get();
        match reference.target() {
            Some(oid) => revwalk.push(oid).unwrap(),
            None => (),
        };
    };
    match revwalk.set_sorting(Sort::TOPOLOGICAL) {
        Ok(()) => (),
        Err(e) => return Err(format!("Error setting the sort of the revwalk: {}", e)),
    };
    let mut oid_list: Vec<Oid> = vec![];
    for commit_oid_result in revwalk {
        match commit_oid_result {
            Ok(oid) => oid_list.push(oid),
            Err(e) => return Err(format!("Error getting oid from revwalk: {}", e)),
        };
    }
    let mut message_list: Vec<String> = vec![];
    for oid in oid_list {
        match repo_temp.find_commit(oid) {
            Ok(commit) => message_list.push(commit.summary().unwrap().parse().unwrap()),
            Err(e) => return Err(format!("Error finding commit in repo: {}", e)),
        };
    }
    Ok(message_list)
}

#[tauri::command]
pub fn git_fetch() -> Result<(), String> {
    let repo_temp_opt;
    unsafe {
        repo_temp_opt = &REPO;
    }
    let repo_temp = match repo_temp_opt {
        Some(repo) => repo,
        None => return Err("No repo to fetch for.".into()),
    };
    let remote_string_array = match repo_temp.remotes() {
        Ok(remote_string_array) => remote_string_array,
        Err(e) => return Err(format!("Error getting array of remotes: {}", e)),
    };
    let empty_refspecs: &[String] = &[];
    for remote_string_opt in remote_string_array.iter() {
        let remote_string = match remote_string_opt {
            Some(remote_string) => remote_string,
            None => return Err("ERROR: A remote string returned None! Possibly due to being non-utf8?".into()),
        };
        let mut remote = match repo_temp.find_remote(remote_string) {
            Ok(remote) => remote,
            Err(e) => return Err(format!("Error finding remote from remote string: {}", e)),
        };
        match remote.fetch(empty_refspecs, None, None) {
            Ok(()) => (),
            Err(e) => return Err(format!("Error fetching: {}", e)),
        };
    }
    println!("Fetch successful!");
    Ok(())
}

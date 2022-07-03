use git2::Repository;
use rfd::AsyncFileDialog;

pub async fn open_repo() -> Option<Repository> {
    let file_handle = AsyncFileDialog::new().set_directory("/").pick_folder().await?;
    match Repository::open(file_handle.path()) {
        Ok(repo) => Some(repo),
        Err(e) => panic!("Repository not found at given path: {}", e),
    }
}

pub fn git_fetch(repo_opt: &Option<Repository>) {
    match repo_opt {
        Some(repo) => {
            repo.find_remote("origin").unwrap().fetch(&["master"], None, None).unwrap();
        },
        None => (),
    }
}

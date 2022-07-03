use git2::Repository;
use git2::Error;
use rfd::AsyncFileDialog;

pub async fn open_repo() -> Result<Option<Repository>, Error> {
    match AsyncFileDialog::new().set_directory("/").pick_folder().await {
        Some(file_handle) => {
            match Repository::open(file_handle.path()) {
                Ok(repo) => Ok(Some(repo)),
                Err(e) => Err(e),
            }
        },
        None => Ok(None),
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

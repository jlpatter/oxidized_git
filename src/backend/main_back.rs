use git2::Repository;
use rfd::AsyncFileDialog;

pub async fn open_repo() -> Option<Repository> {
    let file_handle = AsyncFileDialog::new()
        .set_directory("/")
        .pick_folder()
        .await?;
    let path = file_handle.path().to_str()?;
    match Repository::open(path) {
        Ok(some_repo) => Some(some_repo),
        Err(e) => panic!("Repository not found at given path: {}", e),
    }
}

pub fn git_fetch() {
    println!("Hello World!");
}

use git2::Repository;
use rfd::AsyncFileDialog;

pub async fn open_repo() {
    match AsyncFileDialog::new().set_directory("/").pick_folder().await {
        Some(file_handle) => {
            match Repository::open(file_handle.path()) {
                Ok(repo) => {
                    println!("Do something with repo!");
                },
                Err(e) => panic!("Repository not found at given path: {}", e),
            }
        },
        None => (),
    }
}

pub fn git_fetch() {
    println!("Hello World!");
}

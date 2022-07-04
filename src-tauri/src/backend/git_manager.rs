use git2::Repository;
use rfd::AsyncFileDialog;

static mut REPO: Option<Repository> = None;

#[tauri::command]
pub async fn open_repo() -> String {
    match AsyncFileDialog::new().set_directory("/").pick_folder().await {
        Some(file_handle) => {
            match Repository::open(file_handle.path()) {
                Ok(repo) => {
                    unsafe {
                        REPO = Some(repo);
                    }
                    println!("Repo opened successfully!");
                    "".into()
                },
                Err(e) => e.message().into(),
            }
        },
        None => "".into(),
    }
}

#[tauri::command]
pub fn git_fetch() -> String {
    let repo_temp;
    unsafe {
        repo_temp = &REPO;
    }
    match repo_temp {
        Some(repo) => {
            match repo.find_remote("origin") {
                Ok(mut remote) => {
                    match remote.fetch(&["master"], None, None) {
                        Ok(_) => "".into(),
                        Err(e) => format!("Error fetching: {}", e),
                    }
                },
                Err(e) => format!("Error finding origin: {}", e),
            }
        },
        None => "No repo to fetch for.".into(),
    }
}

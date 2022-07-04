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

// pub fn git_fetch(repo_opt: &Option<Repository>) {
//     match repo_opt {
//         Some(repo) => {
//             repo.find_remote("origin").unwrap().fetch(&["master"], None, None).unwrap();
//         },
//         None => (),
//     }
// }

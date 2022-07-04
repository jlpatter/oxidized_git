use git2::Repository;
use rfd::FileDialog;

static mut REPO: Option<Repository> = None;

#[tauri::command]
pub fn open_repo() -> Result<(), String> {
    match FileDialog::new().set_directory("/").pick_folder() {
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

#[tauri::command]
pub fn git_fetch() -> Result<(), String> {
    let repo_temp;
    unsafe {
        repo_temp = &REPO;
    }
    match repo_temp {
        Some(repo) => {
            match repo.remotes() {
                Ok(remote_string_array) => {
                    for remote_string_opt in remote_string_array.iter() {
                        match remote_string_opt {
                            Some(remote_string) => {
                                match repo.find_remote(remote_string) {
                                    Ok(mut remote) => {
                                        let empty_refspecs: &[String] = &[];
                                        // TODO: Add callback function for authorization!
                                        match remote.fetch(empty_refspecs, None, None) {
                                            Ok(()) => (),
                                            Err(e) => return Err(format!("Error fetching: {}", e)),
                                        }
                                    },
                                    Err(e) => return Err(format!("Error finding remote from remote string: {}", e)),
                                }
                            },
                            None => println!("WARNING: A remote string returned None! Possibly due to being non-utf8?"),
                        };
                    }
                    println!("Successfully completed fetch!");
                    Ok(())
                },
                Err(e) => Err(format!("Error getting array of remotes: {}", e)),
            }
        },
        None => Err("No repo to fetch for.".into()),
    }
}

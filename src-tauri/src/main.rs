#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod git_manager;
pub mod config_manager;
pub mod svg_row;
pub mod parseable_info;

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use tauri::{CustomMenuItem, Manager, Menu, MenuItem, Submenu, Window, WindowBuilder, WindowEvent, Wry};
use tauri::MenuEntry::NativeItem;
use git_manager::GitManager;
use parseable_info::{get_parseable_repo_info, get_files_changed_info_list};

fn handle_error(e: anyhow::Error, main_window: &Window<Wry>) {
    let error_string = format!("{:?}", e);
    main_window.emit_all("error", error_string).unwrap();
}

fn emit_update_all(git_manager: &MutexGuard<GitManager>, main_window: &Window<Wry>) {
    let repo_info_result = get_parseable_repo_info(git_manager);
    match repo_info_result {
        Ok(repo_info_opt) => {
            if let Some(repo_info) = repo_info_opt {
                main_window.emit_all("update_all", repo_info).unwrap();
            } else {
                main_window.emit_all("end-process", "").unwrap();
            }
        },
        Err(e) => handle_error(e, main_window),
    };
}

fn emit_update_changes(git_manager: &MutexGuard<GitManager>, main_window: &Window<Wry>) {
    let changes_info_result = get_files_changed_info_list(git_manager);
    match changes_info_result {
        Ok(changes_info_opt) => {
            if let Some(changes_info) = changes_info_opt {
                main_window.emit_all("update_changes", changes_info).unwrap();
            }
        },
        Err(e) => handle_error(e, main_window),
    }
}

fn main() {
    tauri::Builder::default()
    .setup(|app| {
        let menu;
        if std::env::consts::OS == "macos" {
            menu = Menu::with_items([
                Submenu::new("App", Menu::with_items([
                    CustomMenuItem::new("preferences", "Preferences").into(),
                ])).into(),
                Submenu::new("File", Menu::with_items([
                    CustomMenuItem::new("init", "Init New Repo").into(),
                    CustomMenuItem::new("open", "Open Repo").into(),
                ])).into(),
                Submenu::new("Security", Menu::with_items([
                    CustomMenuItem::new("credentials", "Set Credentials").into(),
                ])).into(),
            ]);
        } else {
            menu = Menu::with_items([
                Submenu::new("File", Menu::with_items([
                    CustomMenuItem::new("init", "Init New Repo").into(),
                    CustomMenuItem::new("open", "Open Repo").into(),
                    NativeItem(MenuItem::Separator),
                    CustomMenuItem::new("preferences", "Preferences").into(),
                ])).into(),
                Submenu::new("Security", Menu::with_items([
                    CustomMenuItem::new("credentials", "Set Credentials").into(),
                ])).into(),
            ]);
        }

        let main_window = WindowBuilder::new(
            app,
            "main-window".to_string(),
            tauri::WindowUrl::App("index.html".into()),
        )
        .menu(menu)
        .maximized(true)
        .title("Oxidized Git")
        .build()?;

        let git_manager_arc: Arc<Mutex<GitManager>> = Arc::new(Mutex::new(GitManager::new()));
        let just_got_repo_arc: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        let just_got_repo_arc_c = just_got_repo_arc.clone();
        main_window.on_window_event(move |event| {
            match event {
                WindowEvent::Focused(is_focused) => {
                    if *is_focused {
                        let mut just_got_repo = just_got_repo_arc_c.lock().unwrap();
                        if *just_got_repo {
                            *just_got_repo = false;
                        } else {
                            main_window_c.emit_all("start-process", "").unwrap();
                            let main_window_c_c = main_window_c.clone();
                            let git_manager_arc_c_c = git_manager_arc_c.clone();
                            thread::spawn(move || {
                                let git_manager = git_manager_arc_c_c.lock().unwrap();
                                emit_update_all(&git_manager, &main_window_c_c);
                            });
                        }
                    }
                },
                _ => {},
            }
        });

        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        let just_got_repo_arc_c = just_got_repo_arc.clone();
        main_window.on_menu_event(move |event| {
            match event.menu_item_id() {
                "preferences" => {
                    let preferences = match config_manager::get_preferences() {
                        Ok(p) => p,
                        Err(e) => {
                            handle_error(e, &main_window_c);
                            return;
                        },
                    };
                    main_window_c.emit_all("show-preferences", preferences).unwrap();
                },
                // Don't use a separate thread for init, open, or clone so as not to break the file dialog in Linux.
                "init" => {
                    main_window_c.emit_all("start-process", "").unwrap();
                    let mut git_manager = git_manager_arc_c.lock().unwrap();
                    let init_result = git_manager.init_repo();
                    match init_result {
                        Ok(did_init) => {
                            if did_init {
                                emit_update_all(&git_manager, &main_window_c);
                                let mut just_got_repo = just_got_repo_arc_c.lock().unwrap();
                                *just_got_repo = true;
                            } else {
                                main_window_c.emit_all("end-process", "").unwrap();
                            }
                        },
                        Err(e) => handle_error(e, &main_window_c),
                    };
                },
                "open" => {
                    main_window_c.emit_all("start-process", "").unwrap();
                    let mut git_manager = git_manager_arc_c.lock().unwrap();
                    let open_result = git_manager.open_repo();
                    match open_result {
                        Ok(did_open) => {
                            if did_open {
                                emit_update_all(&git_manager, &main_window_c);
                                let mut just_got_repo = just_got_repo_arc_c.lock().unwrap();
                                *just_got_repo = true;
                            } else {
                                main_window_c.emit_all("end-process", "").unwrap();
                            }
                        },
                        Err(e) => handle_error(e, &main_window_c),
                    };
                },
                "credentials" => {
                    main_window_c.emit_all("get-credentials", "").unwrap();
                }
                &_ => {},
            };
        });

        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("get-commit-info", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let commit_info_result = git_manager.get_commit_info(s);
                        match commit_info_result {
                            Ok(r) => main_window_c_c.emit_all("commit-info", r).unwrap(),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("reset", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let reset_result = git_manager.git_reset(s);
                        match reset_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("checkout", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let ref_result = git_manager.get_ref_from_name(s);
                        match ref_result {
                            Ok(r) => {
                                let checkout_result = git_manager.git_checkout(&r);
                                match checkout_result {
                                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                                    Err(e) => handle_error(e, &main_window_c_c),
                                };
                            },
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("checkout-remote", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let checkout_result = git_manager.git_checkout_remote(s);
                        match checkout_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("save-preferences", move |event| {
            match event.payload() {
                Some(s) => {
                    match config_manager::save_preferences(s) {
                        Ok(()) => {
                            let main_window_c_c = main_window_c.clone();
                            let git_manager_arc_c_c = git_manager_arc_c.clone();
                            thread::spawn(move || {
                                let git_manager = git_manager_arc_c_c.lock().unwrap();
                                emit_update_all(&git_manager, &main_window_c_c);
                            });
                        },
                        Err(e) => handle_error(e, &main_window_c),
                    };
                },
                None => main_window_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            };
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("save-credentials", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let set_credentials_result = git_manager.set_credentials(s);
                        match set_credentials_result {
                            Ok(()) => (),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("stage", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let stage_result = git_manager.git_stage(s);
                        match stage_result {
                            Ok(()) => emit_update_changes(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("unstage", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let stage_result = git_manager.git_unstage(s);
                        match stage_result {
                            Ok(()) => emit_update_changes(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("file-diff", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let file_diff_result = git_manager.get_file_diff(s);
                        match file_diff_result {
                            Ok(file_lines) => main_window_c_c.emit_all("show-file-lines", file_lines).unwrap(),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("commit", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let commit_result = git_manager.git_commit(s);
                        match commit_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("commit-push", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let commit_result = git_manager.git_commit(s);
                        match commit_result {
                            Ok(()) => {
                                let push_result = git_manager.git_push(None);
                                match push_result {
                                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                                    Err(e) => handle_error(e, &main_window_c_c),
                                };
                            },
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("delete-local-branch", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let delete_result = git_manager.git_delete_local_branch(s);
                        match delete_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("delete-remote-branch", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let delete_result = git_manager.git_delete_remote_branch(s);
                        match delete_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("delete-tag", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let delete_result = git_manager.git_delete_tag(s);
                        match delete_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                }
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("fetch", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c_c.lock().unwrap();
                let fetch_result = git_manager.git_fetch();
                match fetch_result {
                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("pull", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c_c.lock().unwrap();
                let pull_result = git_manager.git_pull();
                match pull_result {
                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("push", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let push_result = git_manager.git_push(Some(s));
                        match push_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("branch", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let branch_result = git_manager.git_branch(s);
                        match branch_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

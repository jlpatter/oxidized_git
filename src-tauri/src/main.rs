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

fn emit_update_all(git_manager: &mut MutexGuard<GitManager>, force_refresh: bool, main_window: &Window<Wry>) {
    let result = get_parseable_repo_info(git_manager, force_refresh);
    match result {
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
    let result = get_files_changed_info_list(git_manager);
    match result {
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
                                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                                emit_update_all(&mut git_manager, false, &main_window_c_c);
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
                    let result = git_manager.init_repo();
                    match result {
                        Ok(did_init) => {
                            if did_init {
                                emit_update_all(&mut git_manager, true, &main_window_c);
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
                    let result = git_manager.open_repo();
                    match result {
                        Ok(did_open) => {
                            if did_open {
                                emit_update_all(&mut git_manager, true, &main_window_c);
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
                        let result = git_manager.get_commit_info(s);
                        match result {
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
        main_window.listen("merge", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_merge(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("rebase", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_rebase(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("cherrypick", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_cherrypick(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("revert", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_revert(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_reset(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_checkout_from_json(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                            Err(e) => handle_error(e, &main_window_c_c),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("checkout-detached-head", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_checkout_detached_head(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_checkout_remote(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                                emit_update_all(&mut git_manager, true, &main_window_c_c);
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
                        let result = git_manager.set_credentials(s);
                        match result {
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
                        let result = git_manager.git_stage_from_json(s);
                        match result {
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
                        let result = git_manager.git_unstage(s);
                        match result {
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
                        let result = git_manager.get_file_diff(s);
                        match result {
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
        main_window.listen("stage-all", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_stage_all();
                match result {
                    Ok(()) => emit_update_changes(&git_manager, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_commit_from_json(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_commit_from_json(s);
                        match result {
                            Ok(()) => {
                                let result_2 = git_manager.git_push(None);
                                match result_2 {
                                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
        main_window.listen("abort", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_abort();
                match result {
                    Ok(()) => emit_update_changes(&git_manager, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("continue-cherrypick", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_continue_cherrypick();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("continue-revert", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_continue_revert();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("continue-merge", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_continue_merge();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("abort-rebase", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_abort_rebase();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("continue-rebase", move |_event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_continue_rebase();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
                    Err(e) => handle_error(e, &main_window_c_c),
                };
            });
        });
        let main_window_c = main_window.clone();
        let git_manager_arc_c = git_manager_arc.clone();
        main_window.listen("discard-changes", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_discard_changes(s);
                        match result {
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
        main_window.listen("delete-local-branch", move |event| {
            let main_window_c_c = main_window_c.clone();
            let git_manager_arc_c_c = git_manager_arc_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_delete_local_branch(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_delete_remote_branch(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_delete_tag(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_fetch();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                let result = git_manager.git_pull();
                match result {
                    Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_push(Some(s));
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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
                        let mut git_manager = git_manager_arc_c_c.lock().unwrap();
                        let result = git_manager.git_branch(s);
                        match result {
                            Ok(()) => emit_update_all(&mut git_manager, false, &main_window_c_c),
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

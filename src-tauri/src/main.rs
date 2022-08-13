#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{CustomMenuItem, Manager, Menu, Submenu, Window, WindowBuilder, Wry};
use backend::git_manager::GitManager;

lazy_static! {
    static ref GIT_MANAGER_ARC: Arc<Mutex<GitManager>> = Arc::new(Mutex::new(GitManager::new()));
}

fn emit_update_all(git_manager: &MutexGuard<GitManager>, temp_main_window: &Window<Wry>) {
    let repo_info_result = git_manager.get_parseable_repo_info();
    match repo_info_result {
        Ok(repo_info) => temp_main_window.emit_all("update_all", repo_info).unwrap(),
        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
    };
}

fn main() {
    tauri::Builder::default()
    .setup(|app| {
        let main_window = WindowBuilder::new(
            app,
            "main-window".to_string(),
            tauri::WindowUrl::App("index.html".into()),
        )
        .menu(
            Menu::with_items([
                Submenu::new("File", Menu::with_items([
                    CustomMenuItem::new("init", "Init New Repo").into(),
                    CustomMenuItem::new("open", "Open Repo").into(),
                ])).into(),
                Submenu::new("Security", Menu::with_items([
                    CustomMenuItem::new("credentials", "Set Credentials").into(),
                ])).into(),
            ])
        )
        .maximized(true)
        .build()?;

        let temp_main_window = main_window.clone();
        main_window.on_menu_event(move |event| {
            match event.menu_item_id() {
                "init" => {
                    let mut git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let init_result = git_manager.init_repo();
                    match init_result {
                        Ok(did_init) => {
                            if did_init {
                                emit_update_all(&git_manager, &temp_main_window);
                            }
                        },
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                "open" => {
                    let mut git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let open_result = git_manager.open_repo();
                    match open_result {
                        Ok(did_open) => {
                            if did_open {
                                emit_update_all(&git_manager, &temp_main_window);
                            }
                        },
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                "credentials" => {
                    temp_main_window.emit_all("get-credentials", "").unwrap();
                }
                &_ => {},
            };
        });

        let temp_main_window = main_window.clone();
        main_window.listen("checkout", move |event| {
            match event.payload() {
                Some(s) => {
                    let git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let ref_result = git_manager.get_ref_from_name(s);
                    match ref_result {
                        Ok(r) => {
                            let checkout_result = git_manager.git_checkout(&r);
                            match checkout_result {
                                Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                            };
                        },
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                None => temp_main_window.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            }
        });
        let temp_main_window = main_window.clone();
        main_window.listen("checkout-remote", move |event| {
            match event.payload() {
                Some(s) => {
                    let git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let checkout_result = git_manager.git_checkout_remote(s);
                    match checkout_result {
                        Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                None => temp_main_window.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            }
        });
        let temp_main_window = main_window.clone();
        main_window.listen("send-credentials", move |event| {
            match event.payload() {
                Some(s) => {
                    let git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let set_credentials_result = git_manager.set_credentials(s);
                    match set_credentials_result {
                        Ok(()) => (),
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                None => temp_main_window.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            }
        });
        let temp_main_window = main_window.clone();
        main_window.listen("refresh", move |_event| {
            let git_manager = GIT_MANAGER_ARC.lock().unwrap();
            emit_update_all(&git_manager, &temp_main_window);
        });
        let temp_main_window = main_window.clone();
        main_window.listen("fetch", move |_event| {
            let git_manager = GIT_MANAGER_ARC.lock().unwrap();
            let fetch_result = git_manager.git_fetch();
            match fetch_result {
                Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
            };
        });
        let temp_main_window = main_window.clone();
        main_window.listen("pull", move |_event| {
            let git_manager = GIT_MANAGER_ARC.lock().unwrap();
            let pull_result = git_manager.git_pull();
            match pull_result {
                Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
            };
        });
        let temp_main_window = main_window.clone();
        main_window.listen("push", move |_event| {
            let git_manager = GIT_MANAGER_ARC.lock().unwrap();
            let push_result = git_manager.git_push();
            match push_result {
                Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
            };
        });
        let temp_main_window = main_window.clone();
        main_window.listen("forcePush", move |_event| {
            let git_manager = GIT_MANAGER_ARC.lock().unwrap();
            let force_push_result = git_manager.git_force_push();
            match force_push_result {
                Ok(()) => emit_update_all(&git_manager, &temp_main_window),
                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
            };
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

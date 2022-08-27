#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use tauri::{CustomMenuItem, Manager, Menu, MenuItem, Submenu, Window, WindowBuilder, Wry};
use tauri::MenuEntry::NativeItem;
use backend::git_manager::GitManager;
use backend::config_manager;
use backend::parseable_info::get_parseable_repo_info;

lazy_static! {
    static ref GIT_MANAGER_ARC: Arc<Mutex<GitManager>> = Arc::new(Mutex::new(GitManager::new()));
}

fn emit_update_all(git_manager: &MutexGuard<GitManager>, temp_main_window: &Window<Wry>) {
    let repo_info_result = get_parseable_repo_info(git_manager);
    match repo_info_result {
        Ok(repo_info) => temp_main_window.emit_all("update_all", repo_info).unwrap(),
        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
    };
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
        .build()?;

        let main_window_c = main_window.clone();
        main_window.on_menu_event(move |event| {
            match event.menu_item_id() {
                "preferences" => {
                    let preferences = match config_manager::get_preferences() {
                        Ok(p) => p,
                        Err(e) => {
                            main_window_c.emit_all("error", e.to_string()).unwrap();
                            return;
                        },
                    };
                    main_window_c.emit_all("show-preferences", preferences).unwrap();
                },
                // Don't use a separate thread for init, open, or clone so as not to break the file dialog in Linux.
                "init" => {
                    let mut git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let init_result = git_manager.init_repo();
                    match init_result {
                        Ok(did_init) => {
                            if did_init {
                                emit_update_all(&git_manager, &main_window_c);
                            }
                        },
                        Err(e) => main_window_c.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                "open" => {
                    let mut git_manager = GIT_MANAGER_ARC.lock().unwrap();
                    let open_result = git_manager.open_repo();
                    match open_result {
                        Ok(did_open) => {
                            if did_open {
                                emit_update_all(&git_manager, &main_window_c);
                            }
                        },
                        Err(e) => main_window_c.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                "credentials" => {
                    main_window_c.emit_all("get-credentials", "").unwrap();
                }
                &_ => {},
            };
        });

        let main_window_c = main_window.clone();
        main_window.listen("checkout", move |event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c.lock().unwrap();
                        let ref_result = git_manager.get_ref_from_name(s);
                        match ref_result {
                            Ok(r) => {
                                let checkout_result = git_manager.git_checkout(&r);
                                match checkout_result {
                                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                                    Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                                };
                            },
                            Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("checkout-remote", move |event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c.lock().unwrap();
                        let checkout_result = git_manager.git_checkout_remote(s);
                        match checkout_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("save-preferences", move |event| {
            match event.payload() {
                Some(s) => {
                    match config_manager::save_preferences(s) {
                        Ok(()) => (),
                        Err(e) => main_window_c.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                None => main_window_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            };
        });
        let main_window_c = main_window.clone();
        main_window.listen("save-credentials", move |event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c.lock().unwrap();
                        let set_credentials_result = git_manager.set_credentials(s);
                        match set_credentials_result {
                            Ok(()) => (),
                            Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                        };
                    },
                    None => main_window_c_c.emit_all("error", "Failed to receive payload from front-end").unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("refresh", move |_event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c.lock().unwrap();
                emit_update_all(&git_manager, &main_window_c_c);
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("fetch", move |_event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c.lock().unwrap();
                let fetch_result = git_manager.git_fetch();
                match fetch_result {
                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                    Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("pull", move |_event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                let git_manager = git_manager_arc_c.lock().unwrap();
                let pull_result = git_manager.git_pull();
                match pull_result {
                    Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                    Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
                };
            });
        });
        let main_window_c = main_window.clone();
        main_window.listen("push", move |event| {
            let git_manager_arc_c = GIT_MANAGER_ARC.clone();
            let main_window_c_c = main_window_c.clone();
            thread::spawn(move || {
                match event.payload() {
                    Some(s) => {
                        let git_manager = git_manager_arc_c.lock().unwrap();
                        let push_result = git_manager.git_push(s);
                        match push_result {
                            Ok(()) => emit_update_all(&git_manager, &main_window_c_c),
                            Err(e) => main_window_c_c.emit_all("error", e.to_string()).unwrap(),
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

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use tauri::{CustomMenuItem, Manager, Menu, Submenu, WindowBuilder};
use backend::git_manager::GitManager;

static mut GIT_MANAGER: GitManager = GitManager::new();

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
            ])
        )
        .inner_size(1280 as f64, 720 as f64)
        .center()
        .build()?;

        let temp_main_window = main_window.clone();
        main_window.on_menu_event(move |event| {
            match event.menu_item_id() {
                "init" => {
                    let init_result;
                    unsafe { init_result = GIT_MANAGER.init_repo(); }
                    match init_result {
                        Ok(did_init) => {
                            if did_init {
                                temp_main_window.emit_all("init", "Init Success").unwrap();
                            }
                        },
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                "open" => {
                    let open_result;
                    unsafe { open_result = GIT_MANAGER.open_repo(); }
                    match open_result {
                        Ok(did_open) => {
                            if did_open {
                                let repo_info_result;
                                unsafe { repo_info_result = GIT_MANAGER.get_parseable_repo_info(); }
                                match repo_info_result {
                                    Ok(repo_info) => temp_main_window.emit_all("open", repo_info).unwrap(),
                                    Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                                };
                            }
                        },
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                &_ => {},
            };
        });

        let temp_main_window = main_window.clone();
        main_window.listen("fetch", move |_event| {
            let fetch_result;
            unsafe { fetch_result = GIT_MANAGER.git_fetch(); }
            match fetch_result {
                Ok(()) => (),
                Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
            }
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

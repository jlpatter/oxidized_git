#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use clipboard::{ClipboardContext, ClipboardProvider};
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
        .maximized(true)
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
                                let repo_info_result;
                                unsafe { repo_info_result = GIT_MANAGER.get_parseable_repo_info(); }
                                match repo_info_result {
                                    Ok(repo_info) => temp_main_window.emit_all("init", repo_info).unwrap(),
                                    Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                                };
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
        let temp_main_window = main_window.clone();
        main_window.listen("copy_to_clipboard", move |event| {
            match event.payload() {
                Some(s) => {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                    ctx.set_contents(s.into()).unwrap();
                },
                None => temp_main_window.emit_all("error", "Failed to copy to clipboard").unwrap(),
            };
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

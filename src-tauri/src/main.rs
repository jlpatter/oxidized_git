#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use tauri::{CustomMenuItem, Manager, Menu, Submenu, WindowBuilder};
use backend::git_manager;

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
                    match git_manager::init_repo() {
                        Ok(()) => temp_main_window.emit_all("init", "Init Success").unwrap(),
                        Err(e) => temp_main_window.emit_all("error", e).unwrap(),
                    };
                },
                "open" => {
                    match git_manager::open_repo() {
                        Ok(()) => {
                            match git_manager::get_all_commit_lines() {
                                Ok(commit_lines) => temp_main_window.emit_all("open", commit_lines).unwrap(),
                                Err(e) => temp_main_window.emit_all("error", e).unwrap(),
                            };
                        },
                        Err(e) => temp_main_window.emit_all("error", e).unwrap(),
                    };
                },
                &_ => {},
            };
        });

        Ok(())
    })
    .invoke_handler(tauri::generate_handler![git_manager::git_fetch])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

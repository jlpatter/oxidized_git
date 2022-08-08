#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend;

use clipboard::{ClipboardContext, ClipboardProvider};
use tauri::{CustomMenuItem, Manager, Menu, Submenu, Window, WindowBuilder, Wry};
use backend::git_manager::GitManager;

static mut GIT_MANAGER: GitManager = GitManager::new();

fn emit_update_all(temp_main_window: &Window<Wry>) {
    let repo_info_result;
    unsafe { repo_info_result = GIT_MANAGER.get_parseable_repo_info(); }
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
                    let init_result;
                    unsafe { init_result = GIT_MANAGER.init_repo(); }
                    match init_result {
                        Ok(did_init) => {
                            if did_init {
                                emit_update_all(&temp_main_window);
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
                                emit_update_all(&temp_main_window);
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
                    let checkout_result;
                    unsafe { checkout_result = GIT_MANAGER.git_checkout(s); }
                    match checkout_result {
                        Ok(()) => emit_update_all(&temp_main_window),
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
                    let checkout_result;
                    unsafe { checkout_result = GIT_MANAGER.git_checkout_remote(s); }
                    match checkout_result {
                        Ok(()) => emit_update_all(&temp_main_window),
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
                    let set_credentials_result;
                    unsafe { set_credentials_result = GIT_MANAGER.set_credentials(s); }
                    match set_credentials_result {
                        Ok(()) => (),
                        Err(e) => temp_main_window.emit_all("error", e.to_string()).unwrap(),
                    };
                },
                None => temp_main_window.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            }
        });
        let temp_main_window = main_window.clone();
        main_window.listen("fetch", move |_event| {
            let fetch_result;
            unsafe { fetch_result = GIT_MANAGER.git_fetch(); }
            match fetch_result {
                Ok(()) => emit_update_all(&temp_main_window),
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
                None => temp_main_window.emit_all("error", "Failed to receive payload from front-end").unwrap(),
            };
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

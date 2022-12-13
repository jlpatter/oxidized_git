use std::sync::{Arc, Mutex};
use std::thread;
use eframe::{App, Frame};
use eframe::egui::{CentralPanel, Color32, Context, Pos2, Stroke, TopBottomPanel};
use rfd::FileDialog;
use crate::git_manager::GitManager;

pub struct OxidizedGitApp {
    git_manager_arc: Arc<Mutex<GitManager>>,
}

impl Default for OxidizedGitApp {
    fn default() -> Self {
        Self {
            git_manager_arc: Arc::new(Mutex::new(GitManager::new())),
        }
    }
}

impl App for OxidizedGitApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("control_panel").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button("Open").clicked() {
                    let path_buf_opt = FileDialog::new().set_directory("/").pick_folder();
                    if let Some(path_buf) = path_buf_opt {
                        let mut git_manager = self.git_manager_arc.lock().unwrap();
                        match git_manager.open_repo(path_buf) {
                            Ok(()) => println!("Open Successful!"),
                            Err(e) => println!("ERROR: {}", e),
                        };
                    }
                }
                if ui.button("Fetch").clicked() {
                    let git_manager_arc_c = self.git_manager_arc.clone();
                    thread::spawn(move || {
                        let git_manager = git_manager_arc_c.lock().unwrap();
                        git_manager.git_fetch().unwrap();
                        // let result = git_manager.git_fetch();
                        // match result {
                        //     Ok(()) => println!("Fetch successful!"),
                        //     Err(e) => println!("ERROR: {}", e),
                        // };
                    });
                }
                if ui.button("Pull").clicked() {
                    println!("Pull button clicked!");
                }
                if ui.button("Push").clicked() {
                    println!("Push button clicked!");
                }
            });
        });
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Oxidized Git");
            let painter = ui.painter();
            painter.line_segment([Pos2::new(125.0, 125.0), Pos2::new(125.0, 200.0)], Stroke::new(5.0, Color32::RED));
            painter.circle_filled(Pos2::new(125.0, 125.0), 10.0, Color32::RED);
            painter.circle_filled(Pos2::new(125.0, 200.0), 10.0, Color32::RED);
        });
    }
}

mod git_manager;
mod config_manager;
mod svg_row;
mod parseable_info;
mod oxidized_git_app;

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use eframe::{egui, NativeOptions};
use git_manager::GitManager;
use oxidized_git_app::OxidizedGitApp;

fn main() {
    let options = NativeOptions {
        initial_window_size: Some(egui::vec2(1028.0, 720.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Oxidized Git",
        options,
        Box::new(|_cc| Box::new(OxidizedGitApp::default())),
    );
}

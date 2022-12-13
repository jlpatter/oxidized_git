use eframe::{App, Frame};
use eframe::egui::{CentralPanel, Color32, Context, Pos2, Stroke, TopBottomPanel};

pub struct OxidizedGitApp {
}

impl Default for OxidizedGitApp {
    fn default() -> Self {
        Self {
        }
    }
}

impl App for OxidizedGitApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("control_panel").show(ctx, |ui| {
            if ui.button("Fetch").clicked() {
                println!("Fetch button clicked!");
            }
            if ui.button("Pull").clicked() {
                println!("Pull button clicked!");
            }
            if ui.button("Push").clicked() {
                println!("Push button clicked!");
            }
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

slint::slint! {
    import { Button } from "std-widgets.slint";
    OGFWindow := Window {
        title: "Orange Git Fish";
        callback fetchBtn-pressed <=> fetchBtn.clicked;
        fetchBtn := Button {
            text: "Fetch";
        }
    }
}

pub fn main() {
    let ogf_window = OGFWindow::new();
    ogf_window.on_fetchBtn_pressed(move || {
        println!("BLURG BLURG");
    });
    ogf_window.run();
}
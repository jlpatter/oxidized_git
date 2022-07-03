use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use crate::backend::main_back;

slint::slint! {
    import { Button } from "std-widgets.slint";
    OGFWindow := Window {
        title: "Orange Git Fish";
        width: 800px;
        height: 600px;
        callback fetchBtn-pressed <=> fetchBtn.clicked;
        callback openBtn-pressed <=> openBtn.clicked;
        GridLayout {
            spacing: 5px;
            Row {
                openBtn := Button {
                    text: "Open";
                    clip: true;
                }
                fetchBtn := Button {
                    text: "Fetch";
                    clip: true;
                }
            }
        }
    }
}

pub fn main() {
    let ogf_window = OGFWindow::new();
    let pool = ThreadPool::new().unwrap();
    ogf_window.on_openBtn_pressed(move || {
        pool.spawn(main_back::open_repo()).expect("Thread failed to spawn!");
        // executor::block_on(main_back::open_repo());
    });
    ogf_window.on_fetchBtn_pressed(move || {
        main_back::git_fetch();
    });
    ogf_window.run();
}

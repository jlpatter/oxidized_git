use futures::executor;
use crate::backend::main_back;

slint::slint! {
    import { Button } from "std-widgets.slint";
    OGFWindow := Window {
        title: "Orange Git Fish";
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
    ogf_window.on_openBtn_pressed(move || {
        let repo_opt = executor::block_on(main_back::open_repo());
        match repo_opt {
            Some(repo) => {
                match repo.path().to_str() {
                    Some(path_str) => println!("{}", path_str),
                    None => (),
                }
            },
            None => (),
        }
    });
    ogf_window.on_fetchBtn_pressed(move || {
        main_back::git_fetch();
    });
    ogf_window.run();
}

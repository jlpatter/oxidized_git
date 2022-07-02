mod frontend;

use frontend::main_window::OGFWindow;
use slint::ComponentHandle;

fn main() {
    OGFWindow::new().run();
}

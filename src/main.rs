mod frontend;
// backend needs to be "mod"ed here so it can be called from the frontend.
// see: https://stackoverflow.com/questions/20922091/how-do-you-use-parent-module-imports-in-rust
mod backend;

use frontend::main_window;

fn main() {
    main_window::main();
}

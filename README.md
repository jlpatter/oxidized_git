# Oxidized Fish

## For Development
### Windows
* Install the Microsoft Visual Studio C++ build tools https://visualstudio.microsoft.com/visual-cpp-build-tools/
* Install WebView2 https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section
* Install Rust https://www.rust-lang.org/tools/install
* Install NodeJS https://nodejs.org/en/
### Mac (Untested)
* Install xcode
* Install homebrew
* Install Rust: https://www.rust-lang.org/tools/install
* Install NodeJS: `brew install node`
### Linux
#### Debian-based distros (Untested)
* Install Tauri dependencies: https://tauri.app/v1/guides/getting-started/prerequisites
* Install Rust: https://www.rust-lang.org/tools/install
* Install NodeJS: `sudo apt install nodejs npm`
#### Arch-based distros
* Install Tauri dependencies: https://tauri.app/v1/guides/getting-started/prerequisites
* Install Rust: https://www.rust-lang.org/tools/install
* Install NodeJS: `sudo pacman -S nodejs npm`
### All
* Create the `oxidized_git/ui/dist` directory (so tauri doesn't get confused when compiling)
* You will need to compile the Rust stuff in the src-tauri directory. I use CLion for development which handles it automatically.
* Run `cargo install tauri-cli`
* Run `npm install` inside the `ui` directory
* Run `cargo tauri dev` in the project root to run the dev environment or `cargo tauri build` to package the application

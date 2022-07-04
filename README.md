# Orange Git Fish

## For Development
### Windows
* Install the Microsoft Visual Studio C++ build tools https://visualstudio.microsoft.com/visual-cpp-build-tools/
* Install WebView2 https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section
* Install Rust https://www.rust-lang.org/tools/install
* Install NodeJS https://nodejs.org/en/
### Mac (Untested)
* Install xcode with this command: `xcode-select --install`
* Install homebrew: `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"`
* Install Rust with this command: `curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh`
* Install NodeJS: `brew install node`
### Linux
#### Debian-based distros (Untested)
* Run this: `sudo apt update && sudo apt install libwebkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev nodejs npm`
* Install Rust with this command: `curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh`
#### Arch-based distros (Untested)
* Run this: `sudo pacman -Syu && sudo pacman -S --needed webkit2gtk base-devel curl wget openssl appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg libvips nodejs npm`
* Install Rust with this command: `curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh`
### All
* Run `npm install` inside the `ui` directory
* Run `cargo tauri dev` in the project root to run the dev environment or `cargo tauri build` to package the application

import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {invoke} from "@tauri-apps/api";
import {exit} from "@tauri-apps/api/process";

class Main {
    run() {
        $('#fetchBtn').click(() => {
            invoke('git_fetch').then((message) => {
                if (message) {
                    console.log(message);
                }
            });
        });

        $('#openBtn').click(() => {
            invoke('open_repo').then((message) => {
                if (message) {
                    console.log(message);
                }
            });
        });

        $('#exitBtn').click(async () => {
            await exit(0);
        });
    }
}

new Main().run();

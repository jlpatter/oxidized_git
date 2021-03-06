import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {invoke} from "@tauri-apps/api";
import {exit} from "@tauri-apps/api/process";

class Main {
    run() {
        $('#fetchBtn').click(() => {
            invoke('git_fetch')
                .then(() => {
                    // TODO: Do something after fetch maybe?
                })
                .catch((error) => console.error(error));
        });

        $('#openBtn').click(() => {
            invoke('open_repo')
                .then(() => {
                    // TODO: Do something after open maybe?
                })
                .catch((error) => console.error(error));
        });

        $('#exitBtn').click(async () => {
            await exit(0);
        });
    }
}

new Main().run();

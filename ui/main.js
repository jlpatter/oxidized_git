import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";
import {exit} from "@tauri-apps/api/process";

class Main {
    run() {
        listen("init", ev => {
            console.log(ev.payload);
        }).then();

        listen("open", ev => {
            console.log(ev.payload);
        }).then();

        listen("error", ev => {
            alert(ev.payload);
        }).then();

        $('#fetchBtn').click(() => {
            emit("fetch").then();
        });

        $('#exitBtn').click(async () => {
            await exit(0);
        });
    }
}

$(window).on('load', () => {
    new Main().run();
});

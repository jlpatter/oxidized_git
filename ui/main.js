import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {invoke} from "@tauri-apps/api";

class Main {
    run() {
        $('#fetchBtn').click(function() {
            invoke('open_repo').then((message) => console.log(message));
        });
    }
}

new Main().run();

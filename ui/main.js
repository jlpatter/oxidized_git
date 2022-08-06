import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";

class Main {
    run() {
        this.showCommitControls();

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
    }

    showCommitControls() {
        $('#commitControls').show();
        $('#mergeControls').hide();
        $('#cherrypickControls').hide();
    }

    showMergeControls() {
        $('#commitControls').hide();
        $('#mergeControls').show();
        $('#cherrypickControls').hide();
    }

    showCherrypickControls() {
        $('#commitControls').hide();
        $('#mergeControls').hide();
        $('#cherrypickControls').show();
    }
}

$(window).on('load', () => {
    new Main().run();
});

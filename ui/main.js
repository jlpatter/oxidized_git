import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";
import {SVGManager} from "./svg_manager";

class Main {
    run() {
        const self = this;
        $('#contextMenu').hide();
        self.showCommitControls();
        self.svgManager = new SVGManager();

        $(window).click(function() {
            $('#contextMenu').hide();
        });

        listen("init", ev => {
            console.log(ev.payload);
        }).then();

        listen("open", ev => {
            console.log(ev.payload);
            self.svgManager.updateCommitTable(ev.payload);
        }).then();

        listen("error", ev => {
            // TODO: Maybe make a modal for errors instead?
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

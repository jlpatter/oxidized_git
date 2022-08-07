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
            self.updateCommitsAndBranches(ev.payload);
        }).then();

        listen("open", ev => {
            self.updateCommitsAndBranches(ev.payload);
        }).then();

        listen("error", ev => {
            // TODO: Maybe make a modal for errors instead?
            alert(ev.payload);
        }).then();

        $('#fetchBtn').click(() => {
            emit("fetch").then();
        });
    }

    updateCommitsAndBranches(repo_info) {
        const self = this;
        self.svgManager.updateCommitTable(repo_info["commit_info_list"]);

        $('#localTableBody tr').remove();
        $('#remoteTableBody tr').remove();
        $('#tagTableBody tr').remove();
        $('#localTableBody').append('<tr><th><h6>Local Branches</h6></th></tr>');
        $('#remoteTableBody').append('<tr><td><h6>Remote Branches</h6></td></tr>');
        $('#tagTableBody').append('<tr><td><h6>Tags</h6></td></tr>');

        repo_info['branch_info_list'].forEach((branchResult) => {
            let branchResultHTML;
            if (branchResult['ahead'] === '0' && branchResult['behind'] === '0') {
                branchResultHTML = '<tr class="unselectable"><td>' + branchResult['branch_name'] + '</td></tr>';
            } else {
                branchResultHTML = '<tr class="unselectable"><td>' + branchResult['branch_name'] + ' ';
                // TODO: These might be backwards.
                if (branchResult['behind'] !== '0') {
                    branchResultHTML += '<span class="right"><i class="bi bi-arrow-down"></i>' + branchResult['behind'] + '</span>';
                }
                if (branchResult['ahead'] !== '0') {
                    branchResultHTML += '<span class="right"><i class="bi bi-arrow-up"></i>' + branchResult['ahead'] + '</span>';
                }
                branchResultHTML += '</td></tr>';
            }
            const $branchResult = $(branchResultHTML);

            if (branchResult['branch_type'] === 'remote') {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_name']);
                });
                $branchResult.on('dblclick', function() {
                    // TODO: Insert git checkout here!
                });
                $('#remoteTableBody').append($branchResult);
            } else if (branchResult['branch_type'] === 'tag') {
                $('#tagTableBody').append($branchResult);
            } else {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_name']);
                });
                $branchResult.on('dblclick', function() {
                    // TODO: Insert git checkout here!
                });
                $('#localTableBody').append($branchResult);
            }
        });
    }

    showContextMenu(event, branchName) {
        const $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        // TODO: Add more branch functionality here!
        const $exampleBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="bi bi-dash-circle"></i> Do Nothing</button>');
        $exampleBtn.click(function() {
            // TODO: Add functionality to the context menu button here!
        });
        $contextMenu.append($exampleBtn);

        $contextMenu.show();
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

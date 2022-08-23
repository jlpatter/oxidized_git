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

        listen("update_all", ev => {
            self.updateCommitsAndBranches(ev.payload);
        }).then();

        listen("get-credentials", ev => {
            $('#credentialsModal').modal('show');
        }).then();

        listen("show-preferences", ev => {
            const $limitCommitsCheckBox = $('#limitCommitsCheckBox'),
                $commitCountNumber = $('#commitCountNumber');
            $limitCommitsCheckBox.prop('checked', ev.payload['limit_commits']);
            $commitCountNumber.val(ev.payload['commit_count']);
            if ($limitCommitsCheckBox.is(':checked')) {
                $commitCountNumber.prop('disabled', false);
            } else {
                $commitCountNumber.prop('disabled', true);
            }
            $('#preferencesModal').modal('show');
        }).then();

        listen("error", ev => {
            // TODO: Maybe make a modal for errors instead?
            alert(ev.payload);
        }).then();

        $('#limitCommitsCheckBox').change(() => {
            if ($('#limitCommitsCheckBox').is(':checked')) {
                $('#commitCountNumber').prop('disabled', false);
            } else {
                $('#commitCountNumber').prop('disabled', true);
            }
        });

        $('#savePreferencesBtn').click(() => {
            emit("save-preferences", {
                limitCommits: $('#limitCommitsCheckBox').is(':checked').toString(),
                commitCount: $('#commitCountNumber').val(),
            }).then();
            $('#preferencesModal').modal('hide');
        });

        $('#saveCredentialsBtn').click(() => {
            const $usernameTxt = $('#usernameTxt'),
                $passwordTxt = $('#passwordTxt');
            emit("save-credentials", {username: $usernameTxt.val(), password: $passwordTxt.val()}).then();
            $usernameTxt.val("");
            $passwordTxt.val("");
            $('#credentialsModal').modal('hide');
        });

        $('#refreshBtn').click(() => {
            emit("refresh").then();
        });

        $('#fetchBtn').click(() => {
            emit("fetch").then();
        });

        $('#pullBtn').click(() => {
            emit("pull").then();
        });

        $('#pushBtn').click(() => {
            emit("push").then();
        });

        $('#forcePushBtn').click(() => {
            emit("forcePush").then();
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
            branchResultHTML = '<tr class="unselectable"><td>';
            if (branchResult['is_head'] === 'true') {
                branchResultHTML += '* ';
            }
            branchResultHTML += branchResult['branch_name'];
            if (branchResult['behind'] !== '0') {
                branchResultHTML += '<span class="right"><i class="bi bi-arrow-down"></i>' + branchResult['behind'] + '</span>';
            }
            if (branchResult['ahead'] !== '0') {
                branchResultHTML += '<span class="right"><i class="bi bi-arrow-up"></i>' + branchResult['ahead'] + '</span>';
            }
            branchResultHTML += '</td></tr>';
            const $branchResult = $(branchResultHTML);

            if (branchResult['branch_type'] === 'remote') {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_name']);
                });
                $branchResult.on('dblclick', function() {
                    emit("checkout-remote", {full_branch_name: branchResult['full_branch_name'], branch_name: branchResult['branch_name']}).then();
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
                    emit("checkout", branchResult['full_branch_name']).then();
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

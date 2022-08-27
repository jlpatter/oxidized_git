import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";
import {SVGManager} from "./svg_manager";

class Main {
    constructor() {
        this.svgManager = new SVGManager();
        this.generalInfo = {};
    }

    run() {
        const self = this;
        $('#contextMenu').hide();
        self.showCommitControls();

        $(window).click(function() {
            $('#contextMenu').hide();
        });

        listen("update_all", ev => {
            self.updateAll(ev.payload);
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

        $('#openPushModalBtn').click(() => {
            if (Object.hasOwn(self.generalInfo, 'head_has_upstream') && self.generalInfo['head_has_upstream'] === 'true') {
                $('#remoteSelect').hide();
            } else {
                $('#remoteSelect').show();
            }
            $('#forcePushCheckBox').prop('checked', false);
            $('#pushModal').modal('show');
        });

        $('#pushBtn').click(() => {
            // Note: By default, pushing will try to use the local branch's upstream first
            // instead of the selected remote from the front-end
            emit("push", {
                selectedRemote: $('#remoteSelect').val(),
                isForcePush: $('#forcePushCheckBox').is(':checked').toString(),
            }).then();
            $('#pushModal').modal('hide');
        });
    }

    updateAll(repo_info) {
        const self = this;
        self.generalInfo = repo_info['general_info'];
        self.svgManager.updateCommitTable(repo_info["commit_info_list"]);
        self.updateFilesChangedInfo(repo_info['files_changed_info_list']);
        self.updateBranchInfo(repo_info["branch_info_list"]);
        self.updateRemoteInfo(repo_info["remote_info_list"]);
    }

    updateFilesChangedInfo(files_changed_info_list) {
        $('#unstagedTableBody tr').remove();
        $('#stagedTableBody tr').remove();
        $('#unstagedTableBody').append('<tr><th><h6>Unstaged Changes</h6></th></tr>');
        $('#stagedTableBody').append('<tr><th><h6>Staged Changes</h6></th></tr>');

        // Unstaged changes
        files_changed_info_list['unstaged_files'].forEach(function(unstagedFile) {
            const $button = $('<button type="button" class="btn btn-success btn-sm right"><i class="bi bi-plus-lg"></i></button>');
            $button.click(function() {
                // TODO: Get this to work!
                alert("Not implemented yet.");
            });
            const $row = $('<tr><td>' + unstagedFile['path'] + '</td></tr>');
            if (unstagedFile['status'] === 2) {  // Deleted
                $row.find('td').prepend('<i class="bi bi-dash-lg"></i> ');
            } else if (unstagedFile['status'] === 3) {  // Modified
                $row.find('td').prepend('<i class="bi bi-pen"></i> ');
            } else if (unstagedFile['status'] === 7) {  // Untracked
                $row.find('td').prepend('<i class="bi bi-plus-lg"></i> ');
            } else if (unstagedFile['status'] === 10) {  // Conflicted
                $row.find('td').prepend('<i class="bi bi-exclamation-diamond"></i> ');
            } else {  // Everything else
                $row.find('td').prepend('<i class="bi bi-question-circle"></i> ');
            }
            $row.find('td').append($button);
            $('#unstagedTableBody').append($row);
        });

        // Staged changes
        files_changed_info_list['staged_files'].forEach(function(stagedFile) {
            const $button = $('<button type="button" class="btn btn-danger btn-sm right"><i class="bi bi-dash-lg"></i></button>');
            $button.click(function() {
                // TODO: Get this to work!
                alert("Not implemented yet.");
            });
            const $row = $('<tr><td>' + stagedFile['path'] + '</td></tr>');
            if (stagedFile['status'] === 2) {  // Deleted
                $row.find('td').prepend('<i class="bi bi-dash-lg"></i> ');
            } else if (stagedFile['status'] === 3) {  // Modified
                $row.find('td').prepend('<i class="bi bi-pen"></i> ');
            } else if (stagedFile['status'] === 1) {  // Added
                $row.find('td').prepend('<i class="bi bi-plus-lg"></i> ');
            } else if (stagedFile['status'] === 10) {  // Conflicted
                $row.find('td').prepend('<i class="bi bi-exclamation-diamond"></i> ');
            } else {  // Everything else
                $row.find('td').prepend('<i class="bi bi-question-circle"></i> ');
            }
            $row.find('td').append($button);
            $('#stagedTableBody').append($row);
        });
    }

    updateBranchInfo(branch_info_list) {
        $('#localTableBody tr').remove();
        $('#remoteTableBody tr').remove();
        $('#tagTableBody tr').remove();
        $('#localTableBody').append('<tr><th><h6>Local Branches</h6></th></tr>');
        $('#remoteTableBody').append('<tr><td><h6>Remote Branches</h6></td></tr>');
        $('#tagTableBody').append('<tr><td><h6>Tags</h6></td></tr>');

        branch_info_list.forEach((branchResult) => {
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

    updateRemoteInfo(remote_info_list) {
        if (remote_info_list.length > 0) {
            const $remoteSelect = $('#remoteSelect');
            $remoteSelect.empty();

            remote_info_list.forEach((remoteResult) => {
                let $option = '';
                if (remoteResult === 'origin') {
                    $option = $('<option value="' + remoteResult + '" selected>' + remoteResult + '</option>');
                } else {
                    $option = $('<option value="' + remoteResult + '">' + remoteResult + '</option>');
                }
                $remoteSelect.append($option);
            });
        }
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

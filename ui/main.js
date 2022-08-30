import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";
import {SVGManager} from "./svg_manager";
import hljs from "highlight.js";

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
            self.clickClearStuff();
        });

        listen("update_all", ev => {
            self.updateAll(ev.payload);
        }).then();

        listen("update_changes", ev => {
            self.updateFilesChangedInfo(ev.payload);
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

        listen("show-file-lines", ev => {
            self.showFileDiff(ev.payload);
        }).then();

        listen("error", ev => {
            // TODO: Maybe make a modal for errors instead?
            alert(ev.payload);
        }).then();

        $('#commits-tab').click(() => {
            self.svgManager.setVisibleCommits();
        });

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

        $('#commitBtn').click(() => {
            const $summaryTxt = $('#summaryTxt'),
                $messageTxt = $('#messageTxt');
            emit("commit", {summaryText: $summaryTxt.val(), messageText: $messageTxt.val()}).then();
            $summaryTxt.val("");
            $messageTxt.val("");
        });

        $('#commitPushBtn').click(() => {
            const $summaryTxt = $('#summaryTxt'),
                $messageTxt = $('#messageTxt');
            emit("commit-push", {summaryText: $summaryTxt.val(), messageText: $messageTxt.val()}).then();
            $summaryTxt.val("");
            $messageTxt.val("");
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

    clickClearStuff() {
        $('#contextMenu').hide();
    }

    showFileDiff(file_lines) {
        const $fileDiffTable = $('#fileDiffTable');

        $fileDiffTable.empty();
        file_lines.forEach((line) => {
            let styleString = 'background-color: transparent;';
            if (line['origin'] === '+') {
                styleString = 'background-color: rgba(0, 255, 0, 0.08);';
            } else if (line['origin'] === '-') {
                styleString = 'background-color: rgba(255, 0, 0, 0.1);';
            }
            let fileLineRow = '<tr style="' + styleString + '"><td class="line-no">';
            if (line['old_lineno'] !== null) {
                fileLineRow += line['old_lineno'];
            }
            fileLineRow += '</td><td class="line-no">';
            if (line['new_lineno'] !== null) {
                fileLineRow += line['new_lineno'];
            }
            fileLineRow += '</td><td>' + line['origin'] + '</td><td class="line-content"><pre><code class="language-' + line['file_type'] + '">' + line['content'] + '</code></pre></td></tr>';
            $fileDiffTable.append($(fileLineRow));
        });
        hljs.highlightAll();
    }

    updateAll(repo_info) {
        const self = this;
        self.generalInfo = repo_info['general_info'];
        self.svgManager.updateCommitTable(repo_info["commit_info_list"]);
        self.updateFilesChangedInfo(repo_info['files_changed_info_list']);
        self.updateBranchInfo(repo_info["branch_info_list"]);
        self.updateRemoteInfo(repo_info["remote_info_list"]);
    }

    prependFileIcon($row, status) {
        if (status === 2) {  // Deleted
            $row.prepend('<i class="bi bi-dash-square-fill" style="color:red;"></i> ');
        } else if (status === 3) {  // Modified
            $row.prepend('<i class="bi bi-pen-fill" style="color:yellow;"></i> ');
        } else if (status === 7 || status === 1) {  // Untracked or Added
            $row.prepend('<i class="bi bi-plus-square-fill" style="color:green;"></i> ');
        } else if (status === 4) {  // Renamed
            $row.prepend('<i class="bi bi-arrow-right-square-fill" style="color:mediumpurple;"></i> ');
        } else if (status === 5) {  // Copied
            $row.prepend('<i class="bi bi-c-square-fill" style="color:green;"></i> ');
        } else if (status === 10) {  // Conflicted
            $row.prepend('<i class="bi bi-exclamation-diamond-fill" style="color:yellow;"></i> ');
        } else {  // Everything else
            $row.prepend('<i class="bi bi-question-diamond-fill" style="color:blue;"></i> ');
        }
    }

    updateFilesChangedInfo(files_changed_info_list) {
        const self = this;

        if (files_changed_info_list['files_changed'] > 0) {
            $('#changes-tab').html('Changes (' + files_changed_info_list['files_changed'] + ')');
        } else {
            $('#changes-tab').html('Changes');
        }

        const $unstagedChanges = $('#unstagedChanges'),
            $stagedChanges = $('#stagedChanges');

        $unstagedChanges.empty();
        $stagedChanges.empty();

        // Unstaged changes
        files_changed_info_list['unstaged_files'].forEach(function(unstagedFile) {
            const $button = $('<button type="button" class="btn btn-success btn-sm right"><i class="bi bi-plus-lg"></i></button>');
            $button.click(function() {
                emit('stage', unstagedFile).then();
            });
            const $row = $('<p class="hoverable-row unselectable">' + unstagedFile['path'] + '</p>');
            self.prependFileIcon($row, unstagedFile['status']);
            $row.append($button);
            $row.click((e) => {
                e.stopPropagation();
                self.clickClearStuff();
                const $selectedRow = $('.selected-row');
                $selectedRow.removeClass('selected-row');
                $selectedRow.addClass('hoverable-row');
                $row.addClass('selected-row');
                $row.removeClass('hoverable-row');
                emit('file-diff', unstagedFile['path']).then();
            });
            $unstagedChanges.append($row);
        });

        // Staged changes
        files_changed_info_list['staged_files'].forEach(function(stagedFile) {
            const $button = $('<button type="button" class="btn btn-danger btn-sm right"><i class="bi bi-dash-lg"></i></button>');
            $button.click(function() {
                emit('unstage', stagedFile).then();
            });
            const $row = $('<p class="hoverable-row unselectable">' + stagedFile['path'] + '</p>');
            self.prependFileIcon($row, stagedFile['status']);
            $row.append($button);
            $row.click((e) => {
                e.stopPropagation();
                self.clickClearStuff();
                const $selectedRow = $('.selected-row');
                $selectedRow.removeClass('selected-row');
                $selectedRow.addClass('hoverable-row');
                $row.addClass('selected-row');
                $row.removeClass('hoverable-row');
                emit('file-diff', stagedFile['path']).then();
            });
            $stagedChanges.append($row);
        });
    }

    updateBranchInfo(branch_info_list) {
        const self = this,
            $localBranches = $('#localBranches'),
            $remoteBranches = $('#remoteBranches'),
            $tags = $('#tags');

        $localBranches.empty();
        $remoteBranches.empty();
        $tags.empty();

        branch_info_list.forEach((branchResult) => {
            let branchResultHTML = '<p class="hoverable-row unselectable">';
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
            branchResultHTML += '</p>';
            const $branchResult = $(branchResultHTML);

            if (branchResult['branch_type'] === 'remote') {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_name']);
                });
                $branchResult.on('dblclick', function() {
                    emit("checkout-remote", {full_branch_name: branchResult['full_branch_name'], branch_name: branchResult['branch_name']}).then();
                });
                $remoteBranches.append($branchResult);
            } else if (branchResult['branch_type'] === 'tag') {
                $tags.append($branchResult);
            } else {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_name']);
                });
                $branchResult.on('dblclick', function() {
                    emit("checkout", branchResult['full_branch_name']).then();
                });
                $localBranches.append($branchResult);
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

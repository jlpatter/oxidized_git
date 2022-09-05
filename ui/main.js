import jQuery from "jquery";
$ = window.$ = window.jQuery = jQuery;
import {emit, listen} from "@tauri-apps/api/event";
import {SVGManager} from "./svg_manager";
import hljs from "highlight.js";

// This doesn't work if it isn't a separate function for some reason...
function togglerClick() {
    this.parentElement.querySelector(".nested").classList.toggle("active");
    this.querySelector(".bi-caret-down-fill").classList.toggle("rotated-caret");
}

class Main {
    constructor() {
        this.processCount = 0;
        this.svgManager = new SVGManager();
        this.generalInfo = {};
    }

    run() {
        const self = this;
        $('#contextMenu').hide();
        self.showCommitControls();

        $('#mainSpinner').hide();

        self.setupTreeViews();

        $(window).click(() => {
            $('#contextMenu').hide();
        });

        $(window).resize(() => {
            self.truncateFilePathText();
        });

        listen("start-process", ev => {
            self.addProcessCount();
        }).then();

        listen("end-process", ev => {
            self.removeProcessCount();
        }).then();

        listen("update_all", ev => {
            self.updateAll(ev.payload);
            self.removeProcessCount();
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
            self.removeProcessCount();
            alert(ev.payload);
        }).then();

        $('#commits-tab').click(() => {
            self.svgManager.setVisibleCommits();
        });

        $('#changes-tab').click(() => {
            self.truncateFilePathText();
        });

        $('#limitCommitsCheckBox').change(() => {
            if ($('#limitCommitsCheckBox').is(':checked')) {
                $('#commitCountNumber').prop('disabled', false);
            } else {
                $('#commitCountNumber').prop('disabled', true);
            }
        });

        $('#savePreferencesBtn').click(() => {
            self.addProcessCount();
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
            self.addProcessCount();
            const $summaryTxt = $('#summaryTxt'),
                $messageTxt = $('#messageTxt');
            emit("commit", {summaryText: $summaryTxt.val(), messageText: $messageTxt.val()}).then();
            $summaryTxt.val("");
            $messageTxt.val("");
        });

        $('#commitPushBtn').click(() => {
            self.addProcessCount();
            const $summaryTxt = $('#summaryTxt'),
                $messageTxt = $('#messageTxt');
            emit("commit-push", {summaryText: $summaryTxt.val(), messageText: $messageTxt.val()}).then();
            $summaryTxt.val("");
            $messageTxt.val("");
        });

        $('#fetchBtn').click(() => {
            self.addProcessCount();
            emit("fetch").then();
        });

        $('#pullBtn').click(() => {
            self.addProcessCount();
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
            self.addProcessCount();
            // Note: By default, pushing will try to use the local branch's upstream first
            // instead of the selected remote from the front-end
            emit("push", {
                selectedRemote: $('#remoteSelect').val(),
                isForcePush: $('#forcePushCheckBox').is(':checked').toString(),
            }).then();
            $('#pushModal').modal('hide');
        });
    }

    setupTreeViews() {
        const toggler = document.getElementsByClassName("parent-tree");

        for (let i = 0; i < toggler.length; i++) {
            toggler[i].addEventListener("click", togglerClick);
        }
    }

    addProcessCount() {
        this.processCount++;
        $('#mainSpinner').show();
    }

    removeProcessCount() {
        this.processCount--;
        if (this.processCount <= 0) {
            $('#mainSpinner').hide();
            // This should only happen when an error occurs on something that doesn't use the spinner
            if (this.processCount < 0) {
                this.processCount = 0;
            }
        }
    }

    unselectAllRows() {
        const $selectedRow = $('.selected-row');
        $selectedRow.removeClass('selected-row');
        $selectedRow.addClass('hoverable-row');
        $('#fileDiffTable').empty();
    }

    selectRow($row) {
        $row.addClass('selected-row');
        $row.removeClass('hoverable-row');
    }

    showFileDiff(file_lines) {
        const $fileDiffTable = $('#fileDiffTable');

        $fileDiffTable.empty();
        file_lines.forEach((line) => {
            let fileLineRow = '<tr><td class="line-no">';
            if (line['origin'] === '+') {
                fileLineRow = '<tr class="added-code-line"><td class="line-no">';
            } else if (line['origin'] === '-') {
                fileLineRow = '<tr class="removed-code-line"><td class="line-no">';
            }
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

    truncateFilePathText() {
        const filePathText = document.getElementsByClassName('file-path-txt');

        for (let i = 0; i < filePathText.length; i++) {
            const txt = filePathText[i],
                shrunkenTxtContainer = txt.parentElement.parentElement;

            // This is so the text can "grow" again.
            txt.textContent = txt.getAttribute('data-original-txt');

            if (txt.clientWidth > 0 && shrunkenTxtContainer.clientWidth > 0) {
                // Set up the text to have ellipsis for width calculations
                if (txt.clientWidth >= shrunkenTxtContainer.clientWidth) {
                    txt.textContent = "..." + txt.textContent;
                }

                while (txt.clientWidth >= shrunkenTxtContainer.clientWidth) {
                    txt.textContent = "..." + txt.textContent.substring(4);
                }
            }
        }
    }

    addFileChangeRow($changesDiv, $button, file, changeType) {
        const self = this,
            // The outer div is the whole row (minus the button), the next inner div is the "unshrunken" text size (i.e. what size the text should fit in), and the last inner div is the size of the text width.
            // This is all used for truncating the text.
            $text = $('<div class="hoverable-row unselectable flex-auto-in-row display-flex-row"><div class="flex-auto-in-row display-flex-row"><div><p class="file-path-txt" data-original-txt="' + file['path'] + '">' + file['path'] + '</p></div></div></div>');
        self.prependFileIcon($text, file['status']);
        $text.click((e) => {
            e.stopPropagation();
            $('#contextMenu').hide();
            self.unselectAllRows();
            self.selectRow($text);
            emit('file-diff', {file_path: file['path'], change_type: changeType}).then();
        });
        const $row = $('<div class="display-flex-row little-padding-bottom"></div>');
        $row.append($text);
        $row.append($button);
        $changesDiv.append($row);
    }

    updateFilesChangedInfo(files_changed_info_list) {
        const self = this;

        self.unselectAllRows();

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
            $button.click(function(e) {
                e.stopPropagation();
                emit('stage', unstagedFile).then();
            });
            self.addFileChangeRow($unstagedChanges, $button, unstagedFile, 'unstaged');
        });

        // Staged changes
        files_changed_info_list['staged_files'].forEach(function(stagedFile) {
            const $button = $('<button type="button" class="btn btn-danger btn-sm right"><i class="bi bi-dash-lg"></i></button>');
            $button.click(function(e) {
                e.stopPropagation();
                emit('unstage', stagedFile).then();
            });
            self.addFileChangeRow($stagedChanges, $button, stagedFile, 'staged');
        });

        self.truncateFilePathText();
    }

    buildBranchResultHTML(branchResult, currentChildren) {
        const self = this;
        let branchResultHTML = '';
        currentChildren.forEach((child) => {
            if (child['children'].length > 0) {
                branchResultHTML += '<li><span class="parent-tree"><i class="bi bi-caret-down-fill"></i> ' + child['text'] + '</span><ul class="nested sub-tree-view">' + self.buildBranchResultHTML(branchResult, child['children']) + '</ul></li>';
            } else {
                branchResultHTML += '<li class="hoverable-row unselectable">';
                if (branchResult['is_head'] === true) {
                    branchResultHTML += '* ';
                }
                branchResultHTML += child['text'];
                if (branchResult['behind'] !== 0) {
                    branchResultHTML += '<span class="right"><i class="bi bi-arrow-down"></i>' + branchResult['behind'] + '</span>';
                }
                if (branchResult['ahead'] !== 0) {
                    branchResultHTML += '<span class="right"><i class="bi bi-arrow-up"></i>' + branchResult['ahead'] + '</span>';
                }
                branchResultHTML += '</li>'
            }
        });
        return branchResultHTML;
    }

    updateBranchInfo(branch_info_list) {
        const self = this,
            $localBranches = $('#localBranches'),
            $remoteBranches = $('#remoteBranches'),
            $tags = $('#tags');

        $localBranches.empty();
        $remoteBranches.empty();
        $tags.empty();

        branch_info_list['branches'].forEach((branchResult) => {
            // Note that the root node's text is blank
            const $branchResult = $(self.buildBranchResultHTML(branchResult, branch_info_list['branch_name_tree']['children']));

            if (branchResult['branch_type'] === 'remote') {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_shorthand']);
                });
                $branchResult.on('dblclick', function() {
                    self.addProcessCount();
                    emit("checkout-remote", {full_branch_name: branchResult['full_branch_name'], branch_shorthand: branchResult['branch_shorthand']}).then();
                });
                $remoteBranches.append($branchResult);
            } else if (branchResult['branch_type'] === 'tag') {
                $tags.append($branchResult);
            } else {
                $branchResult.contextmenu(function(e) {
                    e.preventDefault();
                    self.showContextMenu(e, branchResult['branch_shorthand']);
                });
                $branchResult.on('dblclick', function() {
                    self.addProcessCount();
                    emit("checkout", branchResult['full_branch_name']).then();
                });
                $localBranches.append($branchResult);
            }
        });
        self.setupTreeViews();
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
        const self = this,
            $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        // TODO: Add more branch functionality here!
        const $exampleBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="bi bi-dash-circle"></i> Do Nothing</button>');
        $exampleBtn.click(function() {
            // self.addProcessCount();
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

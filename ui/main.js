import "./import_jquery";
import {emit, listen} from "@tauri-apps/api/event";
import {open} from '@tauri-apps/api/dialog';
import {homeDir} from '@tauri-apps/api/path';
import {SVGManager} from "./svg_manager";
import hljs from "highlight.js";
import Resizable from "resizable";

// This doesn't work if it isn't a separate function for some reason...
function togglerClick() {
    this.parentElement.querySelector(".nested").classList.toggle("active-tree");
    this.querySelector(".fa-caret-down").classList.toggle("rotated-caret");
}

class Main {
    constructor() {
        this.processCount = 0;
        this.svgManager = new SVGManager();
        this.generalInfo = {};
        this.oldSelectedSHA = '';
        this.selectedCommitInfoFilePath = '';
    }

    run() {
        const self = this;
        $('#contextMenu').hide();
        self.showCommitControls();

        $('#mainSpinner').hide();

        self.setupTreeViews();

        self.svgManager.setGraphWidth();

        // Setup resizable columns.
        const resizableColumns = document.querySelectorAll(".resizable-column");
        resizableColumns.forEach((resizableColumn) => {
            const r = new Resizable(resizableColumn, {
                within: 'parent',
                handles: 'e',
                threshold: 10,
                draggable: false,
            });
            if (resizableColumn.classList.contains('resizable-column-file-paths')) {
                r.on('resize', function() {
                    self.truncateFilePathText();
                });
            } else if (resizableColumn.classList.contains('resizable-column-branches')) {
                r.on('resize', function() {
                    self.svgManager.setGraphWidth();
                });
            }
        });

        const resizableRows = document.querySelectorAll(".resizable-row");
        resizableRows.forEach((resizableRow) => {
            const r = new Resizable(resizableRow, {
                within: 'parent',
                handles: 's',
                threshold: 10,
                draggable: false,
            });
            if (resizableRow.classList.contains('resizable-row-graph')) {
                r.on('resize', function() {
                    self.svgManager.setVisibleCommits();
                });
            }
        });

        $(window).click(() => {
            $('#contextMenu').hide();
        });

        $(window).resize(() => {
            self.truncateFilePathText();
            self.svgManager.setVisibleCommits();
            self.svgManager.setGraphWidth();
        });

        listen("start-process", ev => {
            self.addProcessCount();
        }).then();

        listen("end-process", ev => {
            self.removeProcessCount();
        }).then();

        listen("commit-info", ev => {
            self.showCommitInfo(ev.payload);
        }).then();

        listen("update_all", ev => {
            self.updateAll(ev.payload);
            self.removeProcessCount();
        }).then();

        listen("update_changes", ev => {
            self.updateFilesChangedInfo(ev.payload);
        }).then();

        listen("get-clone", ev => {
            $('#cloneModal').modal('show');
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
            self.removeProcessCount();
            // TODO: if removing jQuery usage, 'text(_)' automatically escapes html characters, so that will need to be handled.
            $('#errorMessage').text(ev.payload);
            $('#errorModal').modal('show');
        }).then();

        $('#commits-tab').click(() => {
            self.svgManager.setVisibleCommits();
        });

        $('#changes-tab').click(() => {
            self.truncateFilePathText();
        });

        $('#commit-diff-tab').click(() => {
            self.truncateFilePathText();
        });

        $('#remoteBranchesHeader').contextmenu((e) => {
            e.preventDefault();
            self.showRemoteBranchesHeaderContextMenu(e);
        });

        $('#addRemoteBtn').click(() => {
            const $addRemoteNameTxt = $('#addRemoteNameTxt'),
                $addRemoteURLTxt = $('#addRemoteURLTxt');
            emit("add-remote", {remote_name: $addRemoteNameTxt.val(), remote_url: $addRemoteURLTxt.val()}).then();
            $addRemoteNameTxt.val('');
            $addRemoteURLTxt.val('');
            $('#addRemoteModal').modal('hide');
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
                limit_commits: $('#limitCommitsCheckBox').is(':checked'),
                commit_count: parseInt($('#commitCountNumber').val()),
            }).then();
            $('#preferencesModal').modal('hide');
        });

        $('#clonePathBtn').click(async function() {
            const selected = await open({
                directory: true,
                multiple: false,
                defaultPath: await homeDir(),
            });
            if (selected !== null) {
                $('#clonePathTxt').val(selected);
            }
        });

        $('#cloneBtn').click(() => {
            self.addProcessCount();
            const $cloneURLTxt = $('#cloneURLTxt'),
                $clonePathTxt = $('#clonePathTxt');
            emit("clone", {clone_url: $cloneURLTxt.val(), clone_path: $clonePathTxt.val()}).then();
            $cloneURLTxt.val("");
            $clonePathTxt.val("");
            $('#cloneModal').modal('hide');
        });

        $('#saveCredentialsBtn').click(() => {
            const $usernameTxt = $('#usernameTxt'),
                $passwordTxt = $('#passwordTxt');
            emit("save-credentials", {username: $usernameTxt.val(), password: $passwordTxt.val()}).then();
            $usernameTxt.val("");
            $passwordTxt.val("");
            $('#credentialsModal').modal('hide');
        });

        $('#stageAllBtn').click(() => {
            emit("stage-all").then();
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

        $('#abortCherrypickBtn').click(() => {
            emit("abort").then();
        });

        $('#continueCherrypickBtn').click(() => {
            emit("continue-cherrypick").then();
        });

        $('#cherrypickBtn').click(() => {
            const $cherrypickSha = $('#cherrypickSha');
            emit("cherrypick", {sha: $cherrypickSha.text(), isCommitting: $('#commitCherrypickCheckBox').is(':checked').toString()}).then();
            $('#cherrypickModal').modal('hide');
            $cherrypickSha.text('');
        });

        $('#abortRevertBtn').click(() => {
            emit("abort").then();
        });

        $('#continueRevertBtn').click(() => {
            emit("continue-revert").then();
        });

        $('#revertBtn').click(() => {
            const $revertSha = $('#revertSha');
            emit("revert", {sha: $revertSha.text(), isCommitting: $('#commitRevertCheckBox').is(':checked').toString()}).then();
            $('#revertModal').modal('hide');
            $revertSha.text('');
        });

        $('#abortMergeBtn').click(() => {
            emit("abort").then();
        });

        $('#continueMergeBtn').click(() => {
            emit("continue-merge").then();
        });

        $('#abortRebaseBtn').click(() => {
            emit("abort-rebase").then();
        });

        $('#continueRebaseBtn').click(() => {
            emit("continue-rebase").then();
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

        $('#openBranchModalBtn').click(() => {
            $('#branchCheckoutCheckBox').prop('checked', true);
            $('#branchModal').modal('show');
        });

        $('#branchBtn').click(() => {
            self.addProcessCount();
            const $branchTxt = $('#branchTxt');
            emit("branch", {branch_name: $branchTxt.val(), checkout_on_create: $('#branchCheckoutCheckBox').is(':checked').toString()}).then();
            $branchTxt.val("");
            $('#branchModal').modal('hide');
        });

        $('#deleteLocalBranchBtn').click(() => {
            self.addProcessCount();
            const $branchShorthand = $('#localBranchToDeleteShorthand');
            emit("delete-local-branch", {branch_shorthand: $branchShorthand.text(), delete_remote_branch: $('#deleteRemoteBranchCheckBox').is(':checked').toString()}).then();
            $branchShorthand.text('');
            $('#deleteLocalBranchModal').modal('hide');
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

    selectRow($row, filePath, changeType, sha) {
        const self = this;
        self.unselectAllRows();
        $row.addClass('selected-row');
        $row.removeClass('hoverable-row');
        if (changeType === 'commit') {
            self.selectedCommitInfoFilePath = filePath;
        }
        emit('file-diff', {file_path: filePath, change_type: changeType, sha: sha}).then();
    }

    showFileDiff(file_info) {
        let $fileDiffTable;
        if (file_info['change_type'] === 'commit') {
            $fileDiffTable = $('#commitFileDiffTable');
        } else if (file_info['change_type'] === 'unstaged' || file_info['change_type'] === 'staged') {
            $fileDiffTable = $('#fileDiffTable');
        }

        $fileDiffTable.empty();
        file_info['file_lines'].forEach((line) => {
            let fileLineRow = '<tr><td class="line-no">';
            if (typeof line === 'string') {
                fileLineRow += '</td><td class="line-no"></td><td></td><td class="line-content"><pre><code class="language-plaintext">' + line + '</code></pre></td></tr>';
            } else {
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
            }
            $fileDiffTable.append($(fileLineRow));
        });
        hljs.highlightAll();
    }

    showCommitInfo(commit_info) {
        const self = this,
            $commitInfo = $('#commit-info'),
            $commitChanges = $('#commitChanges'),
            commitWindowInfo = document.getElementById('commitWindowInfo');

        $commitInfo.empty();
        $commitChanges.empty();
        $('#commitFileDiffTable').empty();

        const formattedAuthorTime = new Date(commit_info['author_time'] * 1000).toLocaleString();
        commitWindowInfo.textContent = commit_info['summary'] + ' - ' + commit_info['author_name'] + ' - ' + formattedAuthorTime;

        const formattedCommitterTime = new Date(commit_info['committer_time'] * 1000).toLocaleString();
        const $newCommitInfo = $(
            '<p>' +
            commit_info['sha'] +
            '</p><p style="white-space: pre-wrap;">' +
            commit_info['message'] +
            '</p><table><tr><td>' +
            commit_info['author_name'] +
            '</td><td class="little-padding-left">' +
            commit_info['committer_name'] +
            '</td></tr><tr><td>' +
            formattedAuthorTime +
            '</td><td class="little-padding-left">' +
            formattedCommitterTime +
            '</td></tr></table>'
        );
        $commitInfo.append($newCommitInfo);

        let first = true;
        const textJQueryElements = [];
        commit_info['changed_files'].forEach(function(file) {
            textJQueryElements.push(self.addFileChangeRow($commitChanges, null, file, 'commit', commit_info['sha'], first));
            first = false;
        });

        let foundFileToSelect = false;
        if (self.oldSelectedSHA === commit_info['sha']) {
            const changedFileIndex = commit_info['changed_files'].findIndex(function(file) {
                return file['path'] === self.selectedCommitInfoFilePath;
            });
            if (changedFileIndex !== -1) {
                self.selectRow(textJQueryElements[changedFileIndex], commit_info['changed_files'][changedFileIndex]['path'], 'commit', commit_info['sha']);
                foundFileToSelect = true;
            }
        }
        if (!foundFileToSelect && commit_info['changed_files'].length > 0) {
            self.selectRow(textJQueryElements[0], commit_info['changed_files'][0]['path'], 'commit', commit_info['sha']);
        }
        self.oldSelectedSHA = commit_info['sha'];
        self.truncateFilePathText();
    }

    updateAll(repo_info) {
        const self = this;
        self.updateGeneralInfo(repo_info["general_info"]);
        self.svgManager.updateGraph(repo_info["commit_info_list"], repo_info["general_info"]["head_sha"]);
        self.updateFilesChangedInfo(repo_info['files_changed_info_list']);
        self.updateBranchInfo(repo_info["branch_info_list"]);
        self.updateRemoteInfo(repo_info["remote_info_list"]);
    }

    updateGeneralInfo(general_info) {
        const self = this;
        self.generalInfo = general_info;

        if (self.generalInfo['is_cherrypicking'] === "true") {
            self.showCherrypickControls();
        } else if (self.generalInfo['is_reverting'] === "true") {
            self.showRevertControls();
        } else if (self.generalInfo['is_merging'] === "true") {
            self.showMergeControls();
        } else if (self.generalInfo['is_rebasing'] === "true") {
            self.showRebaseControls();
        } else {
            self.showCommitControls();
        }
    }

    prependFileIcon($row, status) {
        if (status === 2) {  // Deleted
            $row.prepend('<i class="fa-solid fa-square-minus" style="color:red;"></i> ');
        } else if (status === 3) {  // Modified
            $row.prepend('<i class="fa-solid fa-pen" style="color:goldenrod;"></i> ');
        } else if (status === 7 || status === 1) {  // Untracked or Added
            $row.prepend('<i class="fa-solid fa-square-plus" style="color:green;"></i> ');
        } else if (status === 4) {  // Renamed
            $row.prepend('<i class="fa-solid fa-circle-arrow-right" style="color:mediumpurple;"></i> ');
        } else if (status === 5) {  // Copied
            $row.prepend('<i class="fa-regular fa-copy" style="color:green;"></i> ');
        } else if (status === 10) {  // Conflicted
            $row.prepend('<i class="fa-solid fa-triangle-exclamation" style="color:yellow;"></i> ');
        } else {  // Everything else
            $row.prepend('<i class="fa-solid fa-circle-question" style="color:blue;"></i> ');
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

                    // Stop infinite loop from happening if all the text gets filtered out.
                    if (txt.textContent === "...") {
                        break;
                    }
                }
            }
        }
    }

    addFileChangeRow($changesDiv, $button, file, changeType, sha) {
        const self = this,
            // The outer div is the whole row (minus the button), the next inner div is the "unshrunken" text size (i.e. what size the text should fit in), and the last inner div is the size of the text width.
            // This is all used for truncating the text.
            $text = $('<div class="hoverable-row unselectable flex-auto-in-row display-flex-row"><div class="flex-auto-in-row display-flex-row"><div><p class="file-path-txt" data-original-txt="' + file['path'] + '">' + file['path'] + '</p></div></div></div>');
        self.prependFileIcon($text, file['status']);
        $text.click((e) => {
            e.stopPropagation();
            $('#contextMenu').hide();
            self.selectRow($text, file['path'], changeType, sha);
        });
        if (changeType === 'unstaged' || changeType === 'staged') {
            $text.contextmenu((e) => {
                e.preventDefault();
                self.showFileChangeContextMenu(e, file['path'], changeType, file['status']);
            });
        }
        const $row = $('<div class="display-flex-row little-padding-bottom"></div>');
        $row.append($text);
        if ($button !== null) {
            $row.append($button);
        }
        $changesDiv.append($row);
        return $text;
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
            const $button = $('<button type="button" class="btn btn-success btn-sm right"><i class="fa-solid fa-plus"></i></button>');
            $button.click(function(e) {
                e.stopPropagation();
                emit('stage', unstagedFile).then();
            });
            self.addFileChangeRow($unstagedChanges, $button, unstagedFile, 'unstaged', '', false);
        });

        // Staged changes
        files_changed_info_list['staged_files'].forEach(function(stagedFile) {
            const $button = $('<button type="button" class="btn btn-danger btn-sm right"><i class="fa-solid fa-minus"></i></button>');
            $button.click(function(e) {
                e.stopPropagation();
                emit('unstage', stagedFile).then();
            });
            self.addFileChangeRow($stagedChanges, $button, stagedFile, 'staged', '', false);
        });

        self.truncateFilePathText();
    }

    buildBranchResultHTML(currentChildren, $ul, parentTxt) {
        const self = this;
        currentChildren.forEach((child) => {
            if (child['children'].length > 0) {
                const newParentTxt = parentTxt + '-' + child['text'];
                const $nestedList = $('<ul id="' + newParentTxt + '" class="nested sub-tree-view"></ul>');
                self.buildBranchResultHTML(child['children'], $nestedList, newParentTxt);
                const $newListItem = $('<li><span class="parent-tree"><i class="fa-solid fa-caret-down"></i> ' + child['text'] + '</span></li>');
                $newListItem.append($nestedList);
                $ul.append($newListItem);
            } else {
                const $innerListItem = $('<li></li>');
                if (child['branch_info'] !== null) {
                    $innerListItem.addClass('hoverable-row unselectable inner-branch-item');
                }
                let childText = '';
                if (child['branch_info'] !== null) {
                    if (child['branch_info']['is_head'] === true) {
                        childText += '* ';
                    } else if (child['branch_info']['branch_type'] === 'local' && child['branch_info']['has_upstream'] === false) {
                        childText += '<i class="fa-solid fa-triangle-exclamation" style="color:yellow;"></i> ';
                        $innerListItem.attr('data-bs-toggle', 'tooltip');
                        $innerListItem.attr('title', 'This branch has no upstream, consider pushing it!');
                    }
                }
                childText += child['text'];
                $innerListItem.html(childText);
                if (child['branch_info'] !== null) {
                    if (child['branch_info']['behind'] !== 0) {
                        const $behindCount = $('<span class="right"><i class="fa-solid fa-arrow-down"></i>' + child['branch_info']['behind'] + '</span>');
                        $innerListItem.append($behindCount);
                    }
                    if (child['branch_info']['ahead'] !== 0) {
                        const $aheadCount = $('<span class="right"><i class="fa-solid fa-arrow-up"></i>' + child['branch_info']['ahead'] + '</span>');
                        $innerListItem.append($aheadCount);
                    }

                    if (child['branch_info']['branch_type'] === 'remote') {
                        $innerListItem.on('dblclick', function() {
                            self.addProcessCount();
                            emit("checkout-remote", {full_branch_name: child['branch_info']['full_branch_name'], branch_shorthand: child['branch_info']['branch_shorthand']}).then();
                        });
                    } else if (child['branch_info']['branch_type'] === 'local') {
                        $innerListItem.on('dblclick', function() {
                            self.addProcessCount();
                            emit("checkout", child['branch_info']['full_branch_name']).then();
                        });
                    }
                    $innerListItem.click(function() {
                        self.svgManager.scrollToCommit(child['branch_info']['target_sha']);
                    });
                    $innerListItem.contextmenu(function(e) {
                        e.preventDefault();
                        self.showBranchContextMenu(e, child['branch_info']['branch_shorthand'], child['branch_info']['branch_type']);
                    });
                }

                if ($innerListItem.attr('data-bs-toggle') !== undefined) {
                    $innerListItem.tooltip({
                        animation: false,
                    });
                }

                $ul.append($innerListItem);
            }
        });
    }

    updateBranchInfo(branch_info_list) {
        const self = this,
            $localBranches = $('#localBranches'),
            $remoteBranches = $('#remoteBranches'),
            $tags = $('#tags');

        let activeTreeIds = [];
        $('.active-tree').each(function() {
            activeTreeIds.push($(this).attr('id'));
        });

        $localBranches.empty();
        $remoteBranches.empty();
        $tags.empty();

        // The root node is empty, so get its children.
        self.buildBranchResultHTML(branch_info_list['local_branch_info_tree']['children'], $localBranches, "localBranches");
        self.buildBranchResultHTML(branch_info_list['remote_branch_info_tree']['children'], $remoteBranches, "remoteBranches");
        self.buildBranchResultHTML(branch_info_list['tag_branch_info_tree']['children'], $tags, "tags");
        self.setupTreeViews();

        const activeTreeIdsSelector = "#" + activeTreeIds.join(",#");
        $(activeTreeIdsSelector).each(function() {
            $(this).addClass("active-tree");
            $(this).parent().children('.parent-tree').children('.fa-caret-down').addClass('rotated-caret');
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

    showRemoteBranchesHeaderContextMenu(event) {
        const $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        const $addRemoteContextMenuBtn = $('<button type="button" class="btn btn-outline-success btn-sm rounded-0 cm-item"><i class="fa-solid fa-plus"></i> Add Remote</button>');
        $addRemoteContextMenuBtn.click(() => {
            if ($('#remoteBranches:contains("origin")').length === 0) {
                $('#addRemoteNameTxt').val("origin");
            }
            $('#addRemoteModal').modal('show');
        });
        $contextMenu.append($addRemoteContextMenuBtn);

        $contextMenu.show();
    }

    showFileChangeContextMenu(event, path, changeType, status) {
        const $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        const $discardBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-regular fa-trash-can"></i> Discard Changes</button>');
        $discardBtn.click(() => {
            emit("discard-changes", {path: path, change_type: changeType, status: status.toString()}).then();
        });
        $contextMenu.append($discardBtn);

        $contextMenu.show();
    }

    showBranchContextMenu(event, branchShorthand, branchType) {
        const self = this,
            $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        const $deleteBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-regular fa-trash-can"></i> Delete</button>');
        if (branchType === 'local') {
            $deleteBtn.click(() => {
                $('#localBranchToDeleteShorthand').text(branchShorthand);
                $('#deleteRemoteBranchCheckBox').prop('checked', false);
                $('#deleteLocalBranchModal').modal('show');
            });
        } else if (branchType === 'remote') {
            $deleteBtn.click(() => {
                self.addProcessCount();
                emit("delete-remote-branch", branchShorthand).then();
            });
        } else if (branchType === 'tag') {
            $deleteBtn.click(() => {
                self.addProcessCount();
                emit("delete-tag", branchShorthand).then();
            });
        } else {
            $deleteBtn.click(() => {
                alert("Not implemented, sorry!");
            });
        }
        $contextMenu.append($deleteBtn);

        $contextMenu.show();
    }

    showCommitControls() {
        $('#conflictWarningBanner').hide();

        $('#commitControls').show();
        $('#mergeControls').hide();
        $('#rebaseControls').hide();
        $('#cherrypickControls').hide();
        $('#revertControls').hide();
    }

    showMergeControls() {
        $('#conflictWarningBanner').show();

        $('#commitControls').hide();
        $('#mergeControls').show();
        $('#rebaseControls').hide();
        $('#cherrypickControls').hide();
        $('#revertControls').hide();
    }

    showRebaseControls() {
        $('#conflictWarningBanner').show();

        $('#commitControls').hide();
        $('#mergeControls').hide();
        $('#rebaseControls').show();
        $('#cherrypickControls').hide();
        $('#revertControls').hide();
    }

    showCherrypickControls() {
        $('#conflictWarningBanner').show();

        $('#commitControls').hide();
        $('#mergeControls').hide();
        $('#rebaseControls').hide();
        $('#cherrypickControls').show();
        $('#revertControls').hide();
    }

    showRevertControls() {
        $('#conflictWarningBanner').show();

        $('#commitControls').hide();
        $('#mergeControls').hide();
        $('#rebaseControls').hide();
        $('#cherrypickControls').hide();
        $('#revertControls').show();
    }
}

$(window).on('load', () => {
    new Main().run();
});

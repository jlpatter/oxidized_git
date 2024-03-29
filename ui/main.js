import {getVersion} from '@tauri-apps/api/app';
import {writeText} from "@tauri-apps/api/clipboard";
import {open} from '@tauri-apps/api/dialog';
import {emit, listen} from "@tauri-apps/api/event";
import {homeDir} from '@tauri-apps/api/path';
import {relaunch} from '@tauri-apps/api/process';
import {checkUpdate, installUpdate} from '@tauri-apps/api/updater';
import {SVGManager} from "./svg_manager";
import hljs from "highlight.js";
import Resizable from "resizable";

// This doesn't work if it isn't a separate function for some reason...
function togglerClick() {
    this.parentElement.querySelector(".nested").classList.toggle("active-tree");
    this.querySelector(".fa-caret-down").classList.toggle("rotated-caret");
}

class Main {
    SUMMARY_CHAR_SOFT_LIMIT = 50;

    constructor() {
        this.processCount = 0;
        this.svgManager = new SVGManager(this);
        this.generalInfo = {};
        this.oldSelectedSHA = '';
        this.selectedCommitInfoFilePath = '';
        this.selectedFileChangedInfoFilePath = '';
        this.commitFileDiffTableScrollTop = 0;
        this.fileDiffTableScrollTop = 0;
    }

    run() {
        const self = this;
        $('#contextMenu').hide();
        self.showCommitControls();

        $('#mainSpinner').hide();
        $('#updaterSpinner').hide();

        self.setupTreeViews();

        self.svgManager.setGraphWidth();

        self.showWelcomeView();

        $('#summaryTxtCounter').text(self.SUMMARY_CHAR_SOFT_LIMIT.toString());

        checkUpdate().then(async function(updateResult) {
            if (updateResult.shouldUpdate) {
                const updateMessages = updateResult.manifest.body.split(', ');
                updateMessages.forEach((m) => {
                    $('#updateMessages').append($('<li>' + m + '</li>'));
                });
                $('#updateCurrentVersion').text('Current Version: ' + await getVersion());
                $('#updateNewVersion').text('New Version: ' + updateResult.manifest.version);
                $('#updateModal').modal('show');
            }
        }).catch((e) => {
            self.showError(e.toString());
        });

        // Setup file diff tables to only copy content.
        $('#fileDiffTable, #commitFileDiffTable').each(function() {
            $(this).on('copy', function(e) {
                e.preventDefault();
                const text = self.getSelectedText();
                writeText(text).then();
            });
        });

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

        listen("no-open-repo", ev => {
            self.showWelcomeView();
            self.removeProcessCount();
        }).then();

        listen("commit-info", ev => {
            self.showCommitInfo(ev.payload);
        }).then();

        listen("update_all", ev => {
            self.showRepoView();
            self.updateAll(ev.payload);
            self.removeProcessCount();
        }).then();

        listen("update_changes", ev => {
            self.showRepoView();
            self.updateFilesChangedInfo(ev.payload);
        }).then();

        listen("get-init", async function(ev) {
            await self.doInit();
        }).then();

        listen("get-open", async function(ev) {
            await self.doOpen();
        }).then();

        listen("get-clone", ev => {
            $('#cloneModal').modal('show');
        }).then();

        listen("get-credentials", async function(ev) {
            const homePath = await homeDir(),
                sshPubKeyDefaultPath = homePath + ".ssh/id_ed25519.pub",
                sshPrivateKeyDefaultPath = homePath + ".ssh/id_ed25519";
            $('#publicKeyPathTxt').val(sshPubKeyDefaultPath);
            $('#privateKeyPathTxt').val(sshPrivateKeyDefaultPath);
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
            self.showError(ev.payload);
        }).then();

        $('#updateBtn').click(async function() {
            const $updaterSpinner = $('#updaterSpinner');
            $updaterSpinner.show();
            try {
                await installUpdate();
                await relaunch();
            } catch (e) {
                self.showError(e.toString());
            }
            $updaterSpinner.hide();
            $('#updateModal').modal('hide');
        });

        $('#wInitBtn').click(async function() {
            await self.doInit();
        });

        $('#wOpenBtn').click(async function() {
            await self.doOpen();
        });

        $('#wCloneBtn').click(() => {
            $('#cloneModal').modal('show');
        });

        $('#commits-tab').click(() => {
            self.svgManager.setVisibleCommits();
            self.truncateFilePathText();
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

        $('#saveHTTPSBtn').click(() => {
            const $usernameTxt = $('#usernameHTTPSTxt'),
                $passwordTxt = $('#passwordTxt');
            emit("save-https-credentials", {username: $usernameTxt.val(), password: $passwordTxt.val()}).then();
            $usernameTxt.val("");
            $passwordTxt.val("");
            $('#credentialsModal').modal('hide');
        });

        $('#publicKeyPathBtn').click(async function() {
            const selected = await open({
                directory: false,
                multiple: false,
                defaultPath: await homeDir(),
            });
            if (selected !== null) {
                $('#publicKeyPathTxt').val(selected);
            }
        });

        $('#privateKeyPathBtn').click(async function() {
            const selected = await open({
                directory: false,
                multiple: false,
                defaultPath: await homeDir(),
            });
            if (selected !== null) {
                $('#privateKeyPathTxt').val(selected);
            }
        });

        $('#saveSSHBtn').click(() => {
            const $publicKeyPathTxt = $('#publicKeyPathTxt'),
                $privateKeyPathTxt = $('#privateKeyPathTxt'),
                $passphraseTxt = $('#passphraseTxt');
            emit("save-ssh-credentials", {
                public_key_path: $publicKeyPathTxt.val(),
                private_key_path: $privateKeyPathTxt.val(),
                passphrase: $passphraseTxt.val(),
            }).then();
            $publicKeyPathTxt.val("");
            $privateKeyPathTxt.val("");
            $passphraseTxt.val("");
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
            self.updateSummaryTxtCounter();
        });

        $('#commitPushBtn').click(() => {
            self.addProcessCount();
            const $summaryTxt = $('#summaryTxt'),
                $messageTxt = $('#messageTxt');
            emit("commit-push", {summaryText: $summaryTxt.val(), messageText: $messageTxt.val()}).then();
            $summaryTxt.val("");
            $messageTxt.val("");
            self.updateSummaryTxtCounter();
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
            self.addProcessCount();
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

        $('#pushTagBtn').click(() => {
            self.addProcessCount();
            const $tagName = $('#tagName');
            emit("push-tag", {
                tagFullName: $tagName.text(),
                selectedRemote: $('#remoteTagSelect').val(),
                isForcePush: $('#forcePushTagCheckBox').is(':checked').toString(),
            }).then();
            $tagName.text('');
            $('#pushTagModal').modal('hide');
        });

        $('#openStashModalBtn').click(() => {
            $('#stashModal').modal('show');
        });

        $('#stashBtn').click(() => {
            const $stashTxt = $('#stashTxt');
            emit("stash", $stashTxt.val()).then();
            $stashTxt.val('');
            $('#stashModal').modal('hide');
        });

        $('#applyStashBtn').click(() => {
            const $stashIndex = $('#stashIndex');
            emit("apply-stash", {index: $stashIndex.text(), delete_stash: $('#deleteStashCheckBox').is(':checked').toString()}).then();
            $stashIndex.text('');
            $('#applyStashModal').modal('hide');
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

        $('#openTagModalBtn').click(() => {
            $('#tagSha').text('');
            $('#tagModal').modal('show');
        });

        $('#lightweightTagCheckbox').change(function() {
            const $tagMessageContainer = $('#tagMessageContainer');
            if (this.checked) {
                $tagMessageContainer.hide();
            } else {
                $tagMessageContainer.show();
            }
        });

        $('#tagBtn').click(() => {
            self.addProcessCount();
            const $tagSha = $('#tagSha'),
                $lightweightTagCheckbox = $('#lightweightTagCheckbox'),
                $tagTxt = $('#tagTxt'),
                $tagMessageTxt = $('#tagMessageTxt');
            emit("tag", {
                tag_sha: $tagSha.text(),
                is_lightweight: $lightweightTagCheckbox.is(':checked').toString(),
                name: $tagTxt.val(),
                message: $tagMessageTxt.val()
            }).then();
            $tagSha.text('');
            $tagTxt.val('');
            $tagMessageTxt.val('');
            $('#tagModal').modal('hide');
        });

        $('#deleteLocalBranchBtn').click(() => {
            self.addProcessCount();
            const $branchShorthand = $('#localBranchToDeleteShorthand');
            emit("delete-local-branch", {branch_shorthand: $branchShorthand.text(), delete_remote_branch: $('#deleteRemoteBranchCheckBox').is(':checked').toString()}).then();
            $branchShorthand.text('');
            $('#deleteLocalBranchModal').modal('hide');
        });

        $('#summaryTxt').on('input', function() {
            self.updateSummaryTxtCounter();
        });
    }

    getSelectedText() {
        const doc = window.getSelection().getRangeAt(0).cloneContents(),
            nodes = doc.querySelectorAll('tr');
        let text = '';

        if (nodes.length === 0) {
            text = doc.textContent;
        } else {
            [].forEach.call(nodes, function(tr, i) {
                // Get last column's text (since that has the text we want to copy).
                const td = tr.cells[tr.cells.length - 1];
                text += (i ? '\n' : '') + td.textContent;
            });
        }

        return text;
    }

    showError(messageTxt) {
        // TODO: if removing jQuery usage, 'text(_)' automatically escapes html characters, so that will need to be handled.
        $('#errorMessage').text(messageTxt);
        $('#errorModal').modal('show');
    }

    showWelcomeView() {
        $('#repoView').hide();
        $('#welcomeView').show();
    }

    showRepoView() {
        $('#welcomeView').hide();
        $('#repoView').show();
    }

    async doInit() {
        const self = this,
            selected = await open({
            directory: true,
            multiple: false,
            defaultPath: await homeDir(),
        });
        if (selected !== null) {
            self.addProcessCount();
            emit("init", selected).then();
        }
    }

    async doOpen() {
        const self = this,
            selected = await open({
            directory: true,
            multiple: false,
            defaultPath: await homeDir(),
        });
        if (selected !== null) {
            self.addProcessCount();
            emit("open", selected).then();
        }
    }

    updateSummaryTxtCounter() {
        const self = this,
            $summaryTxtCounter = $('#summaryTxtCounter'),
            numOfChars = $('#summaryTxt').val().length,
            remainingNumOfChars = self.SUMMARY_CHAR_SOFT_LIMIT - numOfChars;
        $summaryTxtCounter.text(remainingNumOfChars.toString());
        if (remainingNumOfChars < 0) {
            $summaryTxtCounter.removeClass('text-white');
            $summaryTxtCounter.addClass('text-red');
        } else {
            $summaryTxtCounter.removeClass('text-red');
            $summaryTxtCounter.addClass('text-white');
        }
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

    unselectRows(rowClassToDeselect) {
        const $selectedRow = $('.' + rowClassToDeselect);
        $selectedRow.removeClass('selected-row');
        $selectedRow.addClass('hoverable-row');
        $('#fileDiffTable').empty();
    }

    selectRow($row, rowClassToDeselect, filePath, changeType, sha) {
        const self = this;
        self.unselectRows(rowClassToDeselect);
        $row.addClass('selected-row');
        $row.removeClass('hoverable-row');
        if (changeType === 'commit') {
            self.selectedCommitInfoFilePath = filePath;
        } else if (changeType === 'unstaged' || changeType === 'staged') {
            self.selectedFileChangedInfoFilePath = filePath;
        }
        emit('file-diff', {file_path: filePath, change_type: changeType, sha: sha}).then();
    }

    showFileDiff(file_info) {
        const self = this;
        let $fileDiffTable;
        if (file_info['change_type'] === 'commit') {
            $fileDiffTable = $('#commitFileDiffTable');
        } else if (file_info['change_type'] === 'unstaged' || file_info['change_type'] === 'staged') {
            $fileDiffTable = $('#fileDiffTable');
        }

        $fileDiffTable.empty();
        file_info['file_lines'].forEach((line) => {
            let fileLineRow = '<tr><td class="line-no text-unselectable">';
            if (typeof line === 'string') {
                fileLineRow += '</td><td class="line-no text-unselectable"></td><td class="text-unselectable"></td><td class="line-content"><pre><code class="language-plaintext text-grey">' + line + '</code></pre></td></tr>';
            } else {
                if (line['origin'] === '+') {
                    fileLineRow = '<tr class="added-code-line"><td class="line-no text-unselectable">';
                } else if (line['origin'] === '-') {
                    fileLineRow = '<tr class="removed-code-line"><td class="line-no text-unselectable">';
                }
                if (line['old_lineno'] !== null) {
                    fileLineRow += line['old_lineno'];
                }
                fileLineRow += '</td><td class="line-no text-unselectable">';
                if (line['new_lineno'] !== null) {
                    fileLineRow += line['new_lineno'];
                }
                fileLineRow += '</td><td class="text-unselectable">' + line['origin'] + '</td><td class="line-content"><pre><code class="language-' + line['file_type'] + '">' + line['content'] + '</code></pre></td></tr>';
            }
            $fileDiffTable.append($(fileLineRow));
        });
        hljs.highlightAll();

        if (file_info['change_type'] === 'commit') {
            $('#commitFileDiffTableContainer').scrollTop(self.commitFileDiffTableScrollTop);
            self.commitFileDiffTableScrollTop = 0;
        } else if (file_info['change_type'] === 'unstaged' || file_info['change_type'] === 'staged') {
            $('#fileDiffTableContainer').scrollTop(self.fileDiffTableScrollTop);
            self.fileDiffTableScrollTop = 0;
        }
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
            '</p><table><tr><td><h5>Author</h5></td><td><h5>Committer</h5></td></tr><tr><td>' +
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

        const textJQueryElements = [];
        commit_info['changed_files'].forEach(function(file) {
            textJQueryElements.push(self.addFileChangeRow($commitChanges, null, 'commitChangeFilePath', file, 'commit', commit_info['sha']));
        });

        let foundFileToSelect = false;
        if (self.oldSelectedSHA === commit_info['sha']) {
            const changedFileIndex = commit_info['changed_files'].findIndex(function(file) {
                return file['path'] === self.selectedCommitInfoFilePath;
            });
            if (changedFileIndex !== -1) {
                self.selectRow(textJQueryElements[changedFileIndex], 'commitChangeFilePath', commit_info['changed_files'][changedFileIndex]['path'], 'commit', commit_info['sha']);
                foundFileToSelect = true;
            }
        }
        if (!foundFileToSelect && commit_info['changed_files'].length > 0) {
            self.selectRow(textJQueryElements[0], 'commitChangeFilePath', commit_info['changed_files'][0]['path'], 'commit', commit_info['sha']);
        }
        self.oldSelectedSHA = commit_info['sha'];

        // This is a hacky way of waiting until the flexbox has shrunk before truncating text.
        setTimeout(function() {
            self.truncateFilePathText();
        }, 100);
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

        $('#projectName').text(self.generalInfo['project_name']);

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

    addFileChangeRow($changesDiv, $button, rowClassToDeselect, file, changeType, sha) {
        const self = this,
            // The outer div is the whole row (minus the button)
            // the next inner div is the "unshrunken" text size (i.e. what size the text should fit in)
            // and the last inner div is the size of the text width.
            // This is all used for truncating the text.
            $text = $('<div class="hoverable-row text-unselectable flex-auto-in-row display-flex-row ' + rowClassToDeselect + '"><div class="flex-auto-in-row display-flex-row"><div><p class="file-path-txt" data-original-txt="' + file['path'] + '">' + file['path'] + '</p></div></div></div>');
        self.prependFileIcon($text, file['status']);
        $text.click((e) => {
            e.stopPropagation();
            $('#contextMenu').hide();
            self.selectRow($text, rowClassToDeselect, file['path'], changeType, sha);
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

        self.fileDiffTableScrollTop = $('#fileDiffTableContainer').scrollTop();
        self.unselectRows('changeFilePath');

        if (files_changed_info_list['files_changed'] > 0) {
            $('#changes-tab').html('Changes (' + files_changed_info_list['files_changed'] + ')');
        } else {
            $('#changes-tab').html('Changes');
        }

        const $unstagedChanges = $('#unstagedChanges'),
            $stagedChanges = $('#stagedChanges');

        $unstagedChanges.empty();
        $stagedChanges.empty();

        const textJQueryElements = [];
        // Unstaged changes
        files_changed_info_list['unstaged_files'].forEach(function(unstagedFile) {
            const $button = $('<button type="button" class="btn btn-success btn-sm right"><i class="fa-solid fa-plus"></i></button>');
            $button.click(function(e) {
                e.stopPropagation();
                emit('stage', unstagedFile).then();
            });
            textJQueryElements.push(self.addFileChangeRow($unstagedChanges, $button, 'changeFilePath', unstagedFile, 'unstaged', ''));
        });

        // Staged changes
        files_changed_info_list['staged_files'].forEach(function(stagedFile) {
            const $button = $('<button type="button" class="btn btn-danger btn-sm right"><i class="fa-solid fa-minus"></i></button>');
            $button.click(function(e) {
                e.stopPropagation();
                emit('unstage', stagedFile).then();
            });
            textJQueryElements.push(self.addFileChangeRow($stagedChanges, $button, 'changeFilePath', stagedFile, 'staged', ''));
        });

        let changeType = 'unstaged';
        let changedFileIndex = files_changed_info_list['unstaged_files'].findIndex(function(file) {
            return file['path'] === self.selectedFileChangedInfoFilePath;
        });
        if (changedFileIndex === -1) {
            changeType = 'staged';
            changedFileIndex = files_changed_info_list['staged_files'].findIndex(function(file) {
                return file['path'] === self.selectedFileChangedInfoFilePath;
            });
        }
        if (changedFileIndex !== -1) {
            self.selectRow(textJQueryElements[changedFileIndex], 'changeFilePath', files_changed_info_list[changeType + '_files'][changedFileIndex]['path'], changeType, '');
        }

        // This is a hacky way of waiting until the flexbox has shrunk before truncating text.
        setTimeout(function() {
            self.truncateFilePathText();
        }, 100);
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
                const $innerListItem = $('<li class="display-flex-row"></li>');
                if (child['branch_info'] === null) {
                    $innerListItem.append($('<span class="text-overflow-ellipsis flex-auto-in-row">' + child['text'] + '</span>'));
                } else {
                    $innerListItem.addClass('hoverable-row text-unselectable inner-branch-item');
                    let childText = '';
                    if (child['branch_info']['is_head'] === true) {
                        childText += '* ';
                    } else if (child['branch_info']['branch_type'] === 'local' && child['branch_info']['has_upstream'] === false) {
                        childText += '<i class="fa-solid fa-triangle-exclamation" style="color:yellow;"></i> ';
                        $innerListItem.attr('data-bs-toggle', 'tooltip');
                        $innerListItem.attr('title', 'This branch has no upstream, consider pushing it!');
                    }
                    childText += child['text'];
                    $innerListItem.append($('<span class="text-overflow-ellipsis flex-auto-in-row">' + childText + '</span>'));
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
                        self.svgManager.selectRowViaSha(child['branch_info']['target_sha']);
                    });
                    $innerListItem.contextmenu(function(e) {
                        e.preventDefault();
                        self.showBranchContextMenu(e, child['branch_info']['branch_shorthand'], child['branch_info']['full_branch_name'], child['branch_info']['branch_type'], child['branch_info']['has_upstream']);
                    });

                    if ($innerListItem.attr('data-bs-toggle') !== undefined) {
                        $innerListItem.tooltip({
                            animation: false,
                        });
                    }
                }

                $ul.append($innerListItem);
            }
        });
    }

    updateBranchInfo(branch_info_list) {
        const self = this,
            $localBranches = $('#localBranches'),
            $remoteBranches = $('#remoteBranches'),
            $tags = $('#tags'),
            $stashes = $('#stashes');

        let activeTreeIds = [];
        $('.active-tree').each(function() {
            activeTreeIds.push($(this).attr('id'));
        });

        $localBranches.empty();
        $remoteBranches.empty();
        $tags.empty();
        $stashes.empty();

        // The root node is empty, so get its children.
        self.buildBranchResultHTML(branch_info_list['local_branch_info_tree']['children'], $localBranches, "localBranches");
        self.buildBranchResultHTML(branch_info_list['remote_branch_info_tree']['children'], $remoteBranches, "remoteBranches");
        self.buildBranchResultHTML(branch_info_list['tag_branch_info_tree']['children'], $tags, "tags");

        branch_info_list['stash_info_list'].forEach((stashInfo) => {
            const $stashItem = $('<li class="hoverable-row text-unselectable inner-branch-item"></li>');
            $stashItem.text(stashInfo['message']);
            $stashItem.dblclick(function() {
                self.applyStash(stashInfo['index']);
            });
            $stashItem.contextmenu(function(e) {
                e.preventDefault();
                self.showStashContextMenu(e, stashInfo['index']);
            });
            $stashes.append($stashItem);
        });

        self.setupTreeViews();

        const activeTreeIdsSelector = "#" + activeTreeIds.join(",#");
        $(activeTreeIdsSelector).each(function() {
            $(this).addClass("active-tree");
            $(this).parent().children('.parent-tree').children('.fa-caret-down').addClass('rotated-caret');
        });
    }

    updateRemoteInfo(remote_info_list) {
        if (remote_info_list.length > 0) {
            const $remoteSelect = $('#remoteSelect'),
                $remoteTagSelect = $('#remoteTagSelect');
            $remoteSelect.empty();
            $remoteTagSelect.empty();

            remote_info_list.forEach((remoteResult) => {
                let option = '';
                if (remoteResult === 'origin') {
                    option = '<option value="' + remoteResult + '" selected>' + remoteResult + '</option>';
                } else {
                    option = '<option value="' + remoteResult + '">' + remoteResult + '</option>';
                }
                $remoteSelect.append(option);
                $remoteTagSelect.append(option);
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

    showBranchContextMenu(event, branchShorthand, branchFullName, branchType, hasUpstream) {
        const self = this,
            $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        if (branchType === 'tag') {
            const $pushTagBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-arrow-up"></i> Push Tag</button>')
            $pushTagBtn.click(() => {
                $('#tagName').text(branchFullName);
                $('#forcePushTagCheckBox').prop('checked', false);
                $('#pushTagModal').modal('show');
            });
            $contextMenu.append($pushTagBtn);

            const $deleteBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-regular fa-trash-can"></i> Delete</button>');
            $deleteBtn.click(() => {
                self.addProcessCount();
                emit("delete-tag", branchShorthand).then();
            });
            $contextMenu.append($deleteBtn);
        } else {
            const $deleteBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-regular fa-trash-can"></i> Delete</button>');
            if (branchType === 'local') {
                $deleteBtn.click(() => {
                    const $deleteRemoteBranchCheckBox = $('#deleteRemoteBranchCheckBox');
                    $('#localBranchToDeleteShorthand').text(branchShorthand);
                    $deleteRemoteBranchCheckBox.prop('checked', false);
                    if (hasUpstream !== true) {
                        $deleteRemoteBranchCheckBox.prop('disabled', true);
                    }
                    $('#deleteLocalBranchModal').modal('show');
                });

                const $fastForwardBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-arrow-down"></i> Fast-forward to Remote Branch</button>');
                $fastForwardBtn.click(() => {
                    self.addProcessCount();
                    emit("fast-forward-branch", branchShorthand).then();
                });

                $contextMenu.append($fastForwardBtn);
            } else if (branchType === 'remote') {
                $deleteBtn.click(() => {
                    self.addProcessCount();
                    emit("delete-remote-branch", branchShorthand).then();
                });
            } else {
                $deleteBtn.click(() => {
                    alert("Not implemented, sorry!");
                });
            }
            $contextMenu.append($deleteBtn);
        }

        $contextMenu.show();
    }

    applyStash(stashIndex) {
        $('#stashIndex').text(stashIndex.toString());
        $('#deleteStashCheckBox').prop('checked', true);
        $('#applyStashModal').modal('show');
    }

    showStashContextMenu(event, stashIndex) {
        const self = this,
            $contextMenu = $('#contextMenu');
        $contextMenu.empty();
        $contextMenu.css('left', event.pageX + 'px');
        $contextMenu.css('top', event.pageY + 'px');

        const $applyBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-check"></i> Apply Stash</button>');
        $applyBtn.click(() => {
            self.applyStash(stashIndex);
        });
        $contextMenu.append($applyBtn);

        const $deleteBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-regular fa-trash-can"></i> Delete Stash</button>');
        $deleteBtn.click(() => {
            emit("delete-stash", stashIndex).then();
        });
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

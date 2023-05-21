import {writeText} from "@tauri-apps/api/clipboard";
import Resizable from "resizable";
import {listen} from "@tauri-apps/api/event";

export class RepoView {
    constructor(mainJS) {
        this.mainJS = mainJS;
    }

    setListeners() {
        const self = this;
        listen("update_all", ev => {
            self.showRepoView();
            self.updateAll(ev.payload);
            self.removeProcessCount();
        }).then();

        listen("update_changes", ev => {
            self.showRepoView();
            self.updateFilesChangedInfo(ev.payload);
        }).then();
    }

    setEvents() {
        const self = this;
        // Setup file diff tables to only copy content.
        $('#fileDiffTable, #commitFileDiffTable').each(function() {
            $(this).on('copy', function(e) {
                e.preventDefault();
                const text = self.mainJS.utils.getSelectedText();
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
                    self.mainJS.svgManager.setGraphWidth();
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
                    self.mainJS.svgManager.setVisibleCommits();
                });
            }
        });

        $('#commits-tab').click(() => {
            self.mainJS.svgManager.setVisibleCommits();
            self.truncateFilePathText();
        });

        $('#changes-tab').click(() => {
            self.truncateFilePathText();
        });

        $('#commit-diff-tab').click(() => {
            self.truncateFilePathText();
        });
    }

    showRepoView() {
        $('#welcomeView').hide();
        $('#repoView').show();
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
}
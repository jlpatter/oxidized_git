import {writeText} from "@tauri-apps/api/clipboard";
import {emit} from "@tauri-apps/api/event";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    X_SPACING = 20;  // If changing, be sure to update on backend-end too
    X_OFFSET = 20;  // If changing, be sure to update on backend-end too
    BRANCH_TEXT_SPACING = 5;
    SCROLL_RENDERING_MARGIN = 100;
    /**
     * Constructs the svg manager.
     */
    constructor() {
        this.commitColumn = document.getElementById('commitColumn');
        this.commitTableSVG = document.getElementById('commitTableSVG');
        this.repoInfo = [];
        this.rows = [];
        this.commitsTop = -99;
        this.commitsBottom = -99;
        this.oldRenderingAreaTop = 0;
        this.oldRenderingAreaBottom = 0;
        this.setScrollEvent();
    }

    /**
     * Refreshes the commit table with new entry results.
     */
    updateCommitTable(repoInfo) {
        this.repoInfo = repoInfo;
        this.refreshCommitTable();
    }

    /**
     * Refreshes the commit table. Can be called on its own for a passive refresh.
     */
    refreshCommitTable() {
        const self = this;

        const $textSizeTestContainer = $('<svg width="500" height="500"></svg>');
        const textSizeTest = self.makeSVG('text', {id: 'textSizeTest', x: 0, y: 0, fill: 'white'});
        textSizeTest.textContent = 'A';
        $textSizeTestContainer.append(textSizeTest);
        $('#mainBody').append($textSizeTestContainer);
        const singleCharWidth = textSizeTest.getBBox().width;
        $textSizeTestContainer.remove();

        self.commitTableSVG.setAttribute('height', (self.repoInfo.length * 30).toString());

        self.rows = [];
        const renderingAreaTop = self.oldRenderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN;
        const renderingAreaBottom = self.oldRenderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;

        let maxWidth = 0;
        let currentLocation = 'above';
        for (let i = 0; i < self.repoInfo.length; i++) {
            const commit = self.repoInfo[i];
            const elements = commit['elements'];
            let row = {'pixel_y': commit['pixel_y'], 'elements': []};
            for (const childLine of elements['child_lines']) {
                const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                if (childLine['row-y'] < i) {
                    self.rows[childLine['row-y']]['elements'].unshift(line);
                } else if (childLine['row-y'] === i) {
                    row['elements'].push(line);
                } else {
                    console.error("ERROR: A child line is trying to be drawn after the current node!");
                }
            }
            const circle = self.makeSVG(elements['circle']['tag'], elements['circle']['attrs']);
            row['elements'].push(circle);

            let currentX = -1;
            for (const branch_or_tags of elements['branch_and_tags']) {
                if (currentX === -1) {
                    currentX = (branch_or_tags[0]['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
                }
                branch_or_tags[0]['attrs']['x'] = currentX;
                branch_or_tags[1]['attrs']['x'] = currentX - 5;
                const txtElem = self.makeSVG(branch_or_tags[0]['tag'], branch_or_tags[0]['attrs']);
                const box_width = singleCharWidth * branch_or_tags[0]['textContent'].length + 10;
                branch_or_tags[1]['attrs']['width'] = box_width;
                const rectElem = self.makeSVG(branch_or_tags[1]['tag'], branch_or_tags[1]['attrs']);
                txtElem.textContent = branch_or_tags[0]['textContent'];
                row['elements'].push(rectElem);
                row['elements'].push(txtElem);
                currentX += box_width + self.BRANCH_TEXT_SPACING;
            }

            if (currentX === -1) {
                currentX = (elements['summary_text']['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
            }
            elements['summary_text']['attrs']['x'] = currentX;
            const summaryTxt = self.makeSVG(elements['summary_text']['tag'], elements['summary_text']['attrs']);
            summaryTxt.textContent = elements['summary_text']['textContent'];
            row['elements'].push(summaryTxt);

            let width = currentX + elements['summary_text']['textContent'].length * singleCharWidth;
            elements['back_rect']['attrs']['width'] = width;
            const backRect = self.makeSVG(elements['back_rect']['tag'], elements['back_rect']['attrs']);
            backRect.onclick = self.getClickFunction(commit['sha']);
            backRect.oncontextmenu = self.getContextFunction(commit['sha']);
            row['elements'].push(backRect);

            self.rows.push(row);

            if (currentLocation === 'above' && row['pixel_y'] > renderingAreaTop) {
                self.commitsTop = i;
                currentLocation = 'visible';
            } else if (currentLocation === 'visible' && row['pixel_y'] > renderingAreaBottom) {
                self.commitsBottom = i - 1;
                currentLocation = 'below';
            }
            maxWidth = Math.max(maxWidth, width);
        }

        // If commitsBottom wasn't set then the graph doesn't reach the bottom of the screen,
        // so set commitsBottom to the last commit
        if (self.commitsBottom === -99) {
            self.commitsBottom = self.rows.length - 1;
        }

        self.renderVisibleCommits();
        self.commitTableSVG.setAttribute('width', maxWidth.toString());
    }

    renderVisibleCommits() {
        const self = this;

        let df = document.createDocumentFragment();
        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            for (const element of self.rows[i]['elements']) {
                df.appendChild(element);
            }
        }

        self.commitTableSVG.innerHTML = '';

        self.commitTableSVG.appendChild(df);
    }

    // This is used particularly for switching tabs.
    setVisibleCommits() {
        const self = this;
        if (self.rows.length > 0) {
            self.commitsTop = 0;

            const renderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN,
                renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;
            let isInRenderingArea = false;
            while (!isInRenderingArea) {
                if (self.rows[self.commitsTop]['pixel_y'] < renderingAreaTop) {
                    self.commitsTop++;
                } else {
                    isInRenderingArea = true;
                }
            }

            self.commitsBottom = self.commitsTop;
            isInRenderingArea = false;
            while (!isInRenderingArea) {
                if (self.commitsBottom + 1 < self.rows.length && self.rows[self.commitsBottom + 1]['pixel_y'] < renderingAreaBottom) {
                    self.commitsBottom++;
                } else {
                    isInRenderingArea = true;
                }
            }

            self.renderVisibleCommits();
            self.oldRenderingAreaTop = renderingAreaTop;
            self.oldRenderingAreaBottom = renderingAreaBottom;
        }
    }

    // This assumes that scrollTop hasn't changed
    setVisibleCommitsOnResize() {
        const self = this;
        if (self.rows.length > 0) {
            const renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;

            if (renderingAreaBottom < self.oldRenderingAreaBottom) {
                // Rendering area has shrunk
                let isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsBottom - 1 >= 0 && self.rows[self.commitsBottom]['pixel_y'] > renderingAreaBottom) {
                        self.commitsBottom--;
                    } else {
                        isInRenderingArea = true;
                    }
                }
            } else if (renderingAreaBottom > self.oldRenderingAreaBottom) {
                // Rendering area has grown
                let isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsBottom + 1 < self.rows.length && self.rows[self.commitsBottom + 1]['pixel_y'] < renderingAreaBottom) {
                        self.commitsBottom++;
                    } else {
                        isInRenderingArea = true;
                    }
                }
            }

            self.renderVisibleCommits();
            self.oldRenderingAreaBottom = renderingAreaBottom;
        }
    }

    setScrollEvent() {
        const self = this;
        self.commitColumn.addEventListener('scroll', () => {
            if (self.rows.length > 0) {
                const renderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN,
                    renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;
                if (renderingAreaTop < self.oldRenderingAreaTop) {
                    // Scrolling Up
                    // Remove visible rows that are below the rendering area
                    let isInRenderingArea = false;
                    while (!isInRenderingArea) {
                        if (self.commitsBottom - 1 >= 0 && self.rows[self.commitsBottom]['pixel_y'] > renderingAreaBottom) {
                            self.commitsBottom--;
                        } else {
                            isInRenderingArea = true;
                        }
                    }

                    // Add above rows to rendering area (if they're present there)
                    isInRenderingArea = false;
                    while (!isInRenderingArea) {
                        if (self.commitsTop - 1 >= 0 && self.rows[self.commitsTop - 1]['pixel_y'] > renderingAreaTop) {
                            self.commitsTop--;
                        } else {
                            isInRenderingArea = true;
                        }
                    }
                } else if (renderingAreaTop > self.oldRenderingAreaTop) {
                    // Scrolling down
                    // Remove visible rows that are above the rendering area
                    let isInRenderingArea = false;
                    while (!isInRenderingArea) {
                        if (self.rows[self.commitsTop]['pixel_y'] < renderingAreaTop) {
                            self.commitsTop++;
                        } else {
                            isInRenderingArea = true;
                        }
                    }

                    // Add below rows to rendering area (if they're present there)
                    isInRenderingArea = false;
                    while (!isInRenderingArea) {
                        if (self.commitsBottom + 1 < self.rows.length && self.rows[self.commitsBottom + 1]['pixel_y'] < renderingAreaBottom) {
                            self.commitsBottom++;
                        } else {
                            isInRenderingArea = true;
                        }
                    }
                }

                self.renderVisibleCommits();
                self.oldRenderingAreaTop = renderingAreaTop;
                self.oldRenderingAreaBottom = renderingAreaBottom;
            }
        });
    }

    /**
     * Makes an SVG element
     * @param {string} tag
     * @param {Object<string, number|string>} attrs
     * @return {SVGElement|SVGGraphicsElement}
     */
    makeSVG(tag, attrs) {
        const el = document.createElementNS('http://www.w3.org/2000/svg', tag);
        // eslint-disable-next-line guard-for-in
        for (const k in attrs) {
            el.setAttribute(k, attrs[k]);
        }
        return el;
    }

    unselectAllRows() {
        const svgRowElements = document.querySelectorAll('.svg-selected-row');
        svgRowElements.forEach((svgRowElement) => {
            svgRowElement.classList.remove('svg-selected-row');
            svgRowElement.classList.add('svg-hoverable-row');
        });

        $('#commit-info').empty();
        $('#commitChanges').empty();
        $('#commitFileDiffTable').empty();
    }

    selectRow(row) {
        row.classList.add('svg-selected-row');
        row.classList.remove('svg-hoverable-row');
    }

    getClickFunction(sha) {
        const self = this;
        return function(event) {
            self.unselectAllRows();
            self.selectRow(event.target);
            // Will call start-process from back-end
            emit("get-commit-info", sha).then();
        };
    }

    /**
     * Gets the function to be called by oncontextmenu
     * @return {(function(*): void)|*}
     */
    getContextFunction(sha) {
        return function(event) {
            event.preventDefault();
            const $contextMenu = $('#contextMenu');
            $contextMenu.empty();
            $contextMenu.css('left', event.pageX + 'px');
            $contextMenu.css('top', event.pageY + 'px');

            const $mergeBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-code-merge"></i> Merge</button>');
            $mergeBtn.click(function() {
                emit("merge", sha).then();
            });
            $contextMenu.append($mergeBtn);

            const $cherrypickBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-bullseye"></i> Cherrypick Commit</button>');
            $cherrypickBtn.click(function() {
                $('#cherrypickSha').text(sha);
                $('#commitCherrypickCheckBox').prop('checked', true);
                $('#cherrypickModal').modal('show');
            });
            $contextMenu.append($cherrypickBtn);

            const $revertBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-rotate-left"></i> Revert Commit</button>');
            $revertBtn.click(function() {
                $('#revertSha').text(sha);
                $('#commitRevertCheckBox').prop('checked', true);
                $('#revertModal').modal('show');
            });
            $contextMenu.append($revertBtn);

            const $copyShaBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-regular fa-clipboard"></i> Copy SHA</button>');
            $copyShaBtn.click(function() {
                writeText(sha).then();
            });
            $contextMenu.append($copyShaBtn);

            const $softResetBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-solid fa-rotate-left"></i> Soft Reset to Here</button>');
            $softResetBtn.click(function() {
                emit("reset", {sha: sha, type: "soft"}).then();
            });
            $contextMenu.append($softResetBtn);

            const $mixedResetBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-solid fa-rotate-left"></i> Mixed Reset to Here</button>');
            $mixedResetBtn.click(function() {
                emit("reset", {sha: sha, type: "mixed"}).then();
            });
            $contextMenu.append($mixedResetBtn);

            const $hardResetBtn = $('<button type="button" class="btn btn-outline-danger btn-sm rounded-0 cm-item"><i class="fa-solid fa-rotate-left"></i> Hard Reset to Here</button>');
            $hardResetBtn.click(function() {
                emit("reset", {sha: sha, type: "hard"}).then();
            });
            $contextMenu.append($hardResetBtn);

            $contextMenu.show();
        };
    }
}

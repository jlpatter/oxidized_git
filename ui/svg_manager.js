import {writeText} from "@tauri-apps/api/clipboard";
import {emit} from "@tauri-apps/api/event";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    Y_SPACING = 24;  // If changing, be sure to update on backend-end too
    Y_OFFSET = 20;  // If changing, be sure to update on backend-end too
    BRANCH_TEXT_SPACING = 5;
    RIGHT_TEXT_SPACING = 10;
    SCROLL_RENDERING_MARGIN = 100;
    SCROLLBAR_WIDTH = 12;  // If changing, be sure to update in CSS!
    /**
     * Constructs the svg manager.
     */
    constructor(mainJS) {
        this.commitColumn = document.getElementById('commitColumn');
        this.commitTableSVG = document.getElementById('commitTableSVG');
        this.rows = [];
        this.commitsTop = -99;
        this.commitsBottom = -99;
        this.selectedSHA = '';
        this.mainJS = mainJS;
        this.setScrollEvent();
    }

    getSingleCharWidth() {
        const self = this,
            $textSizeTestContainer = $('<svg width="500" height="500"></svg>'),
            textSizeTest = self.makeSVG('text', {id: 'textSizeTest', x: 0, y: 0, fill: 'white'});
        textSizeTest.textContent = 'A';
        $textSizeTestContainer.append(textSizeTest);
        $('#mainBody').append($textSizeTestContainer);
        const singleCharWidth = textSizeTest.getBBox().width;
        $textSizeTestContainer.remove();
        return singleCharWidth;
    }

    /**
     * Refreshes the commit table. Can be called on its own for a passive refresh.
     */
    updateGraph(commitsInfo, headSHA) {
        const self = this,
            singleCharWidth = self.getSingleCharWidth();

        for (let i = 0; i < self.rows.length; i++) {
            self.removeBranchLabels(self.rows[i]);
        }

        const graphWidth = Number(self.commitTableSVG.getAttribute('width'));
        if (commitsInfo['svg_row_draw_properties'].length > 0) {
            self.rows = [];

            for (let i = 0; i < commitsInfo['svg_row_draw_properties'].length; i++) {
                const commit = commitsInfo['svg_row_draw_properties'][i];
                const elements = commit['elements'];
                let row = {'sha': commit['sha'], 'pixel_y': commit['pixel_y'], 'lines': [], 'branches': [], 'circle': null, 'summaryTxt': null, 'authorName': null, 'authorTime': null, 'backRect': null};
                for (const childLine of elements['child_lines']) {
                    const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                    if (childLine['row-y'] < i) {
                        self.rows[childLine['row-y']]['lines'].push(line);
                    } else if (childLine['row-y'] === i) {
                        row['lines'].push(line);
                    } else {
                        console.error("ERROR: A child line is trying to be added after the current node!");
                    }
                }
                row['circle'] = self.makeSVG(elements['circle']['tag'], elements['circle']['attrs']);

                const summaryTxt = self.makeSVG(elements['summary_text']['tag'], elements['summary_text']['attrs']);
                summaryTxt.textContent = elements['summary_text']['textContent'];
                row['summaryTxt'] = summaryTxt;

                const authorTimeX = graphWidth - (elements['author_time']['textContent'].length * singleCharWidth) - self.RIGHT_TEXT_SPACING;
                elements['author_time']['attrs']['x'] = authorTimeX;
                const authorTime = self.makeSVG(elements['author_time']['tag'], elements['author_time']['attrs']);
                authorTime.textContent = elements['author_time']['textContent'];
                row['authorTime'] = authorTime;

                elements['author_name']['attrs']['x'] = authorTimeX - (elements['author_name']['textContent'].length * singleCharWidth) - self.RIGHT_TEXT_SPACING;
                const authorName = self.makeSVG(elements['author_name']['tag'], elements['author_name']['attrs']);
                authorName.textContent = elements['author_name']['textContent'];
                row['authorName'] = authorName;

                elements['back_rect']['attrs']['width'] = graphWidth - elements['circle']['attrs']['cx'];
                const backRect = self.makeSVG(elements['back_rect']['tag'], elements['back_rect']['attrs']);
                backRect.onclick = self.getClickFunction(commit['sha']);
                backRect.ondblclick = self.getDblClickFunction(commit['sha']);
                backRect.oncontextmenu = self.getContextFunction(commit['sha']);
                row['backRect'] = backRect;

                self.rows.push(row);
            }
        }

        self.addBranchLabels(commitsInfo['branch_draw_properties'], singleCharWidth);

        self.setVisibleCommits();
        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.selectRowOnRefresh(headSHA);
    }

    selectRowOnRefresh(headSHA) {
        const self = this;
        let selectedIndex = 0;
        let foundOldSelected = false;
        if (self.selectedSHA !== '') {
            const tempIndex = self.rows.findIndex(function(row) {
                return row['sha'] === self.selectedSHA;
            });
            if (tempIndex !== -1) {
                selectedIndex = tempIndex;
                foundOldSelected = true;
            } else {
                self.selectedSHA = '';
            }
        }
        if (!foundOldSelected && headSHA !== '') {
            const tempIndex = self.rows.findIndex(function(row) {
                return row['sha'] === headSHA;
            });
            if (tempIndex !== -1) {
                selectedIndex = tempIndex;
            }
        }
        if (selectedIndex >= 0 && selectedIndex < self.rows.length) {
            self.selectRow(self.rows[selectedIndex]['backRect'], self.rows[selectedIndex]['sha']);
        }
    }

    addBranchLabels(branchDrawProperties, singleCharWidth) {
        const self = this;

        for (let i = 0; i < branchDrawProperties.length; i++) {
            const rowIndex = self.rows.findIndex(function (row) {
                return row['sha'] === branchDrawProperties[i][0];
            });
            if (rowIndex !== -1) {
                const summaryTxtElem = self.rows[rowIndex]['summaryTxt'];
                const pixel_y = Number(self.rows[rowIndex]['circle'].getAttribute('cy'));
                let currentPixelX = Number(summaryTxtElem.getAttribute('x'));
                for (let j = 0; j < branchDrawProperties[i][1].length; j++) {
                    const branch = branchDrawProperties[i][1][j];
                    branch[0]['attrs']['x'] = currentPixelX;
                    branch[0]['attrs']['y'] += pixel_y;
                    const txtElem = self.makeSVG(branch[0]['tag'], branch[0]['attrs']);
                    const box_width = singleCharWidth * branch[0]['textContent'].length + 10;

                    branch[1]['attrs']['x'] = currentPixelX - 5;
                    branch[1]['attrs']['y'] += pixel_y;
                    branch[1]['attrs']['width'] = box_width;
                    const rectElem = self.makeSVG(branch[1]['tag'], branch[1]['attrs']);
                    txtElem.textContent = branch[0]['textContent'];

                    self.rows[rowIndex]['branches'].push(rectElem);
                    self.rows[rowIndex]['branches'].push(txtElem);

                    currentPixelX += box_width + self.BRANCH_TEXT_SPACING;
                    summaryTxtElem.setAttribute('x', currentPixelX.toString());
                }
            }
        }
    }

    removeBranchLabels(row) {
        if (row['branches'].length > 0) {
            const startX = Number(row['branches'][1].getAttribute('x'));
            row['branches'] = [];
            row['summaryTxt'].setAttribute('x', startX.toString());
        }
    }

    renderVisibleCommits() {
        const self = this;

        let df = document.createDocumentFragment();
        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            self.rows[i]['lines'].forEach((line) => {
                df.appendChild(line);
            });
        }

        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            df.appendChild(self.rows[i]['circle']);
            df.appendChild(self.rows[i]['summaryTxt']);
            df.appendChild(self.rows[i]['authorName']);
            df.appendChild(self.rows[i]['authorTime']);
            self.rows[i]['branches'].forEach((branch) => {
                df.appendChild(branch);
            });
            df.appendChild(self.rows[i]['backRect']);
        }

        self.commitTableSVG.innerHTML = '';

        self.commitTableSVG.appendChild(df);
    }

    setGraphWidth() {
        const self = this,
            singleCharWidth = self.getSingleCharWidth(),
            newGraphWidth = $('#mainBody').width() - self.commitTableSVG.getBoundingClientRect().left - self.SCROLLBAR_WIDTH;

        self.commitTableSVG.setAttribute('width', newGraphWidth.toString());
        for (let i = 0; i < self.rows.length; i++) {
            const authorTimeX = newGraphWidth - (self.rows[i]['authorTime'].textContent.length * singleCharWidth) - self.RIGHT_TEXT_SPACING;
            self.rows[i]['authorTime'].setAttribute('x', authorTimeX.toString());
            self.rows[i]['authorName'].setAttribute('x', (authorTimeX - (self.rows[i]['authorName'].textContent.length * singleCharWidth) - self.RIGHT_TEXT_SPACING).toString());
            self.rows[i]['backRect'].setAttribute('width', (newGraphWidth - Number(self.rows[i]['circle'].getAttribute('cx'))).toString());
        }
    }

    setVisibleCommits() {
        const self = this;
        if (self.rows.length > 0) {
            const renderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN,
                renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;

            // Convert from pixels to index.
            self.commitsTop = Math.max(Math.round((renderingAreaTop - self.Y_OFFSET) / self.Y_SPACING), 0);
            self.commitsBottom = Math.min(Math.round((renderingAreaBottom - self.Y_OFFSET) / self.Y_SPACING), self.rows.length - 1);

            self.renderVisibleCommits();
        }
    }

    scrollToCommit(sha) {
        const self = this;
        if (sha !== '') {
            const rowIndex = self.rows.findIndex(function(row) {
                return row['sha'] === sha;
            });
            if (rowIndex !== -1) {
                const rowPixelY = rowIndex * self.Y_SPACING + self.Y_OFFSET;
                const halfClientHeight = self.commitColumn.clientHeight / 2;
                // scrollTop automatically bounds itself for negative numbers or numbers greater than the max scroll position.
                self.commitColumn.scrollTop = rowPixelY - halfClientHeight;
                self.setVisibleCommits();
            }
        }
    }

    setScrollEvent() {
        const self = this;
        self.commitColumn.addEventListener('scroll', () => {
            self.setVisibleCommits();
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

    selectRow(backRectElement, sha) {
        const self = this;
        self.unselectAllRows();
        backRectElement.classList.add('svg-selected-row');
        backRectElement.classList.remove('svg-hoverable-row');
        self.selectedSHA = sha;
        // Will call start-process from back-end
        emit("get-commit-info", sha).then();
    }

    getClickFunction(sha) {
        const self = this;
        return function(event) {
            self.selectRow(event.target, sha);
        };
    }

    getDblClickFunction(sha) {
        return function(event) {
            emit("checkout-detached-head", sha).then();
        }
    }

    /**
     * Gets the function to be called by oncontextmenu
     * @return {(function(*): void)|*}
     */
    getContextFunction(sha) {
        const self = this;
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

            const $rebaseBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="fa-solid fa-database"></i> Rebase Onto Here</button>');
            $rebaseBtn.click(function() {
                self.mainJS.addProcessCount();
                emit("rebase", sha).then();
            });
            $contextMenu.append($rebaseBtn);

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

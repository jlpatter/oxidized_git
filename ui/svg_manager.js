import {writeText} from "@tauri-apps/api/clipboard";
import {emit} from "@tauri-apps/api/event";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    Y_SPACING = 24;
    Y_OFFSET = 20;
    X_SPACING = 15;
    X_OFFSET = 20;
    LINE_STROKE_WIDTH = 2;
    BRANCH_TEXT_SPACING = 5;
    SCROLL_RENDERING_MARGIN = 100;
    /**
     * Constructs the svg manager.
     */
    constructor() {
        this.commitColumn = document.getElementById('commitColumn');
        this.commitTableSVG = document.getElementById('commitTableSVG');
        this.rows = [];
        this.commitsTop = -99;
        this.commitsBottom = -99;
        this.setScrollEvent();
    }

    getSingleCharWidth() {
        const self = this;
        const $textSizeTestContainer = $('<svg width="500" height="500"></svg>');
        const textSizeTest = self.makeSVG('text', {id: 'textSizeTest', x: 0, y: 0, fill: 'white'});
        textSizeTest.textContent = 'A';
        $textSizeTestContainer.append(textSizeTest);
        $('#mainBody').append($textSizeTestContainer);
        const singleCharWidth = textSizeTest.getBBox().width;
        $textSizeTestContainer.remove();

        return singleCharWidth;
    }

    updateCommitTable(commitsInfo) {
        const self = this;

        const singleCharWidth = self.getSingleCharWidth();

        if (commitsInfo['clear_entire_old_graph']) {
            self.rows = [];
        }

        for (let i = 0; i < self.rows.length; i++) {
            self.removeBranchLabels(self.rows[i], singleCharWidth);
        }

        if (!commitsInfo['clear_entire_old_graph'] && commitsInfo['deleted_shas'].length > 0) {
            self.removeRows(commitsInfo['deleted_shas']);
        }

        const newRows = [];
        let maxWidth = Number(self.commitTableSVG.getAttribute('width'));
        for (let i = 0; i < commitsInfo['created_commit_info_list'].length; i++) {
            const commit = commitsInfo['created_commit_info_list'][i];
            let row = {'sha': commit['sha'], 'childShas': commit['child_shas'], 'parentShas': commit['parent_shas'], 'circle': null, 'summaryTxt': null, 'backRect': null, 'lines': [], 'branches': []};

            // TODO: Insert logic for adding row elements here! (without proper x positions yet)
            // TODO: Delete all(?) lines and re-add them

            newRows.push(row);
            // const width = Number(summaryElement.getAttribute('x')) + summaryElement.textContent.length * singleCharWidth;
            // maxWidth = Math.max(maxWidth, width);
        }

        // TODO: Add logic for updating y positions based on index in the rows array.
        // TODO: Add logic for updating x positions based on a 'hashmap' of the occupied spaces.

        self.rows = newRows.concat(self.rows);

        maxWidth = self.addBranchLabels(commitsInfo['branch_draw_properties'], singleCharWidth, maxWidth);

        self.setVisibleCommits();
        self.commitTableSVG.setAttribute('width', maxWidth.toString());
        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
    }

    addBranchLabels(branchDrawProperties, singleCharWidth, maxWidth) {
        const self = this;

        for (let i = 0; i < branchDrawProperties.length; i++) {
            const rowIndex = self.rows.findIndex(function(row) {
                return row['sha'] === branchDrawProperties[i][0];
            });
            if (rowIndex !== -1) {
                const summaryTxtElem = self.rows[rowIndex]['summaryTxt'];
                const backRectElem = self.rows[rowIndex]['backRect'];
                const pixel_y = Number(self.rows[rowIndex]['circle'].getAttribute('cx'));
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
                backRectElem.setAttribute('width', (currentPixelX + summaryTxtElem.textContent.length * singleCharWidth).toString());
                maxWidth = Math.max(maxWidth, currentPixelX + summaryTxtElem.textContent.length * singleCharWidth);
            }
        }
        return maxWidth;
    }

    removeBranchLabels(row, singleCharWidth) {
        if (row['branches'].length > 0) {
            // row['branches'][1] gets the first branch label's txtElem.
            const startingPixelX = Number(row['branches'][1].getAttribute('x'));

            row['branches'] = [];

            row['summaryTxt'].setAttribute('x', startingPixelX.toString());
            row['backRect'].setAttribute('width', (startingPixelX + row['summaryTxt'].textContent.length * singleCharWidth).toString());
        }
    }

    get_color_string(x) {
        let color_num = x % 4;
        if (color_num === 0) {
            return "#00CC19";
        } else if (color_num === 1) {
            return "#0198A6";
        } else if (color_num === 2) {
            return "#FF7800";
        } else {
            return "#FF0D00";
        }
    }

    removeRows(shas) {
        const self = this;

        shas.forEach((sha) => {
            const indexToDelete = self.rows.findIndex(function(row) {
                return row['sha'] === sha;
            });

            if (indexToDelete === -1) {
                console.error("Couldn't find row to remove from graph!");
            } else {
                const tempParentShas = self.rows[indexToDelete]['parentShas'];

                self.rows.splice(indexToDelete, 1);

                let parentIndexes = tempParentShas.map(function(parentSha) {
                    return self.rows.findIndex(function(row) {
                        return row['sha'] === parentSha;
                    });
                }).filter(function(parentIndex) { return parentIndex !== -1; });

                // Remove the line coming from the parent commit(s)
                for (let i = 0; i < parentIndexes.length; i++) {
                    const lineIndexToRemove = self.rows[parentIndexes[i]]['lines'].findIndex(function(line) {
                        return line['target-sha'] === sha;
                    });
                    if (lineIndexToRemove !== -1) {
                        self.rows[parentIndexes[i]]['lines'].splice(lineIndexToRemove, 1);
                    }
                }
            }
        });
    }

    renderVisibleCommits() {
        const self = this;

        let df = document.createDocumentFragment();
        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            for (const element of self.rows[i]['lines'].map(function(line) {return line['element'];})) {
                df.appendChild(element);
            }
        }

        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            for (const element of self.rows[i]['branches']) {
                df.appendChild(element);
            }
            df.appendChild(self.rows[i]['circle']);
            df.appendChild(self.rows[i]['summaryTxt']);
            df.appendChild(self.rows[i]['backRect']);
        }

        self.commitTableSVG.innerHTML = '';

        self.commitTableSVG.appendChild(df);
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

import {writeText} from "@tauri-apps/api/clipboard";
import {emit} from "@tauri-apps/api/event";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    Y_SPACING = 24;  // If changing, be sure to update on backend-end too
    Y_OFFSET = 20;  // If changing, be sure to update on backend-end too
    X_SPACING = 15;  // If changing, be sure to update on backend-end too
    X_OFFSET = 20;  // If changing, be sure to update on backend-end too
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
        for (let i = 0; i < commitsInfo['svg_row_draw_properties'].length; i++) {
            const commit = commitsInfo['svg_row_draw_properties'][i];
            const elements = commit['elements'];
            let row = {'sha': commit['sha'], 'pixel_y': commit['pixel_y'], 'lines': [], 'branches': []};
            for (const childLine of elements['child_lines']) {
                const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                if (childLine['row-y'] < i) {
                    newRows[childLine['row-y']]['lines'].push(line);
                } else if (childLine['row-y'] === i) {
                    row['lines'].push(line);
                } else {
                    console.error("ERROR: childLine tried to be added after a row!");
                }
            }
            row['circle'] = self.makeSVG(elements['circle']['tag'], elements['circle']['attrs']);

            let currentX = (elements['summary_text']['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
            elements['summary_text']['attrs']['x'] = currentX;
            const summaryTxt = self.makeSVG(elements['summary_text']['tag'], elements['summary_text']['attrs']);
            summaryTxt.textContent = elements['summary_text']['textContent'];
            row['summaryTxt'] = summaryTxt;

            let width = currentX + elements['summary_text']['textContent'].length * singleCharWidth;
            elements['back_rect']['attrs']['width'] = width;
            const backRect = self.makeSVG(elements['back_rect']['tag'], elements['back_rect']['attrs']);
            backRect.onclick = self.getClickFunction(commit['sha']);
            backRect.ondblclick = self.getDblClickFunction(commit['sha']);
            backRect.oncontextmenu = self.getContextFunction(commit['sha']);
            row['backRect'] = backRect;

            newRows.push(row);
            maxWidth = Math.max(maxWidth, width);
        }

        self.addRows(newRows);

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
                let currentPixelX = Number(summaryTxtElem.getAttribute('x'));
                for (let j = 0; j < branchDrawProperties[i][1].length; j++) {
                    const branch = branchDrawProperties[i][1][j];
                    branch[0]['attrs']['x'] = currentPixelX;
                    branch[0]['attrs']['y'] += self.rows[rowIndex]['pixel_y'];
                    const txtElem = self.makeSVG(branch[0]['tag'], branch[0]['attrs']);
                    const box_width = singleCharWidth * branch[0]['textContent'].length + 10;

                    branch[1]['attrs']['x'] = currentPixelX - 5;
                    branch[1]['attrs']['y'] += self.rows[rowIndex]['pixel_y'];
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

    addRows(newRows) {
        const self = this,
            startIndex = newRows.length,
            amountToMove = self.Y_SPACING * newRows.length;

        let pixel_y = amountToMove + self.Y_OFFSET;
        self.rows = newRows.concat(self.rows);

        for (let i = startIndex; i < self.rows.length; i++) {
            self.rows[i]['pixel_y'] = pixel_y;
            self.moveYAttributes(self.rows[i]['lines'], amountToMove);
            self.moveYAttributes(self.rows[i]['branches'], amountToMove);
            self.moveYAttributes([self.rows[i]['circle'], self.rows[i]['summaryTxt'], self.rows[i]['backRect']], amountToMove);
            pixel_y += self.Y_SPACING;
        }

        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.setVisibleCommits();
    }

    removeRows(shas) {
        // TODO: Figure out why it's deleting the entire graph!
        console.log(shas);
        const self = this;

        const indexesToRemove = [];
        for (let i = 0; i < self.rows.length; i++) {
            if (shas.includes(self.rows[i]['sha'])) {
                indexesToRemove.push(i);
            }
        }

        // Remove from bottom up so the spacing doesn't go weird.
        indexesToRemove.reverse();

        indexesToRemove.forEach((i) => {
            const pixelY = self.rows[i]['pixel_y'];
            self.rows.splice(i, 1);
            if (i < self.rows.length) {
                self.rows[i]['pixel_y'] = pixelY;
                self.moveYAttributes(self.rows[i]['lines'], -self.Y_SPACING);
                self.moveYAttributes(self.rows[i]['branches'], -self.Y_SPACING);
                self.moveYAttributes([self.rows[i]['circle'], self.rows[i]['summaryTxt'], self.rows[i]['backRect']], -self.Y_SPACING);
            }
        });

        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.setVisibleCommits();
    }

    moveYAttributes(elements, amountToMove) {
        for (let j = 0; j < elements.length; j++) {
            if (elements[j].hasAttribute('y1')) {
                const new_y1 = Number(elements[j].getAttribute('y1')) + amountToMove;
                elements[j].setAttribute('y1', new_y1.toString());
            }
            if (elements[j].hasAttribute('y2')) {
                const new_y1 = Number(elements[j].getAttribute('y2')) + amountToMove;
                elements[j].setAttribute('y2', new_y1.toString());
            }
            if (elements[j].hasAttribute('d')) {
                // This assumes 'd' is structured like the following: "M x1 y1 C x2 y2, x3 y3, x4 y4"
                const oldD = elements[j].getAttribute('d').split(', ');
                const firstElemSplit = oldD.shift().split(' C ');
                const firstPair = firstElemSplit[0].slice(2).split(' ');
                const secondPair = firstElemSplit[1].split(' ');
                const thirdPair = oldD[0].split(' ');
                const fourthPair = oldD[1].split(' ');
                const newD = 'M ' +
                    firstPair[0] + ' ' +
                    (Number(firstPair[1]) + amountToMove).toString() +
                    ' C ' +
                    secondPair[0] + ' ' +
                    (Number(secondPair[1]) + amountToMove).toString() + ', ' +
                    thirdPair[0] + ' ' +
                    (Number(thirdPair[1]) + amountToMove).toString() + ', ' +
                    fourthPair[0] + ' ' +
                    (Number(fourthPair[1]) + amountToMove).toString();
                elements[j].setAttribute('d', newD);
            }
            if (elements[j].hasAttribute('cy')) {
                const new_y1 = Number(elements[j].getAttribute('cy')) + amountToMove;
                elements[j].setAttribute('cy', new_y1.toString());
            }
            if (elements[j].hasAttribute('y')) {
                const new_y1 = Number(elements[j].getAttribute('y')) + amountToMove;
                elements[j].setAttribute('y', new_y1.toString());
            }
        }
    }

    renderVisibleCommits() {
        const self = this;

        let df = document.createDocumentFragment();
        for (let i = self.commitsTop; i <= self.commitsBottom; i++) {
            for (const element of self.rows[i]['lines']) {
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

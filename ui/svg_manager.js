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

    /**
     * Refreshes the commit table with new entry results.
     */
    updateCommitTable(repoInfo) {
        this.refreshCommitTable(repoInfo);
    }

    /**
     * Refreshes the commit table.
     */
    refreshCommitTable(repoInfo) {
        const self = this;

        const $textSizeTestContainer = $('<svg width="500" height="500"></svg>');
        const textSizeTest = self.makeSVG('text', {id: 'textSizeTest', x: 0, y: 0, fill: 'white'});
        textSizeTest.textContent = 'A';
        $textSizeTestContainer.append(textSizeTest);
        $('#mainBody').append($textSizeTestContainer);
        const singleCharWidth = textSizeTest.getBBox().width;
        $textSizeTestContainer.remove();

        self.commitTableSVG.setAttribute('height', ((repoInfo.length + 1) * self.Y_SPACING).toString());

        self.rows = [];

        let maxWidth = 0;
        for (let i = 0; i < repoInfo.length; i++) {
            const commit = repoInfo[i];
            const elements = commit['elements'];
            let row = {'sha': commit['sha'], 'pixel_y': commit['pixel_y'], 'elements': []};
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
            backRect.ondblclick = self.getDblClickFunction(commit['sha']);
            backRect.oncontextmenu = self.getContextFunction(commit['sha']);
            row['elements'].push(backRect);

            self.rows.push(row);
            maxWidth = Math.max(maxWidth, width);
        }

        self.setVisibleCommits();
        self.commitTableSVG.setAttribute('width', maxWidth.toString());
    }

    removeRow(sha) {
        const self = this;
        const commitIndex = self.rows.findIndex(function(row) {
            return row['sha'] === sha;
        });
        let pixel_y = self.rows[commitIndex]['pixel_y'];
        self.rows.splice(commitIndex, 1);

        for (let i = commitIndex; i < self.rows.length; i++) {
            self.rows[i]['pixel_y'] = pixel_y;
            for (let j = 0; j < self.rows[i]['elements'].length; j++) {
                if (self.rows[i]['elements'][j].hasAttribute('y1')) {
                    const new_y1 = Number(self.rows[i]['elements'][j].getAttribute('y1')) - self.Y_SPACING;
                    self.rows[i]['elements'][j].setAttribute('y1', new_y1.toString());
                }
                if (self.rows[i]['elements'][j].hasAttribute('y2')) {
                    const new_y1 = Number(self.rows[i]['elements'][j].getAttribute('y2')) - self.Y_SPACING;
                    self.rows[i]['elements'][j].setAttribute('y2', new_y1.toString());
                }
                if (self.rows[i]['elements'][j].hasAttribute('d')) {
                    // This assumes 'd' is structured like the following: "M x1 y1 C x2 y2, x3 y3, x4 y4, x5 y5, x6 y6"
                    const oldD = self.rows[i]['elements'][j].getAttribute('d').split(', ');
                    const firstElemSplit = oldD.shift().split(' C ');
                    const firstPair = firstElemSplit[0].slice(2).split(' ');
                    const secondPair = firstElemSplit[1].split(' ');
                    const thirdPair = oldD[0].split(' ');
                    const fourthPair = oldD[1].split(' ');
                    const fifthPair = oldD[2].split(' ');
                    const sixthPair = oldD[3].split(' ');
                    const newD = 'M ' +
                        firstPair[0] + ' ' +
                        (Number(firstPair[1]) - self.Y_SPACING).toString() +
                        ' C ' +
                        secondPair[0] + ' ' +
                        (Number(secondPair[1]) - self.Y_SPACING).toString() + ', ' +
                        thirdPair[0] + ' ' +
                        (Number(thirdPair[1]) - self.Y_SPACING).toString() + ', ' +
                        fourthPair[0] + ' ' +
                        (Number(fourthPair[1]) - self.Y_SPACING).toString() + ', ' +
                        fifthPair[0] + ' ' +
                        (Number(fifthPair[1]) - self.Y_SPACING).toString() + ', ' +
                        sixthPair[0] + ' ' +
                        (Number(sixthPair[1]) - self.Y_SPACING).toString();
                    self.rows[i]['elements'][j].setAttribute('d', newD);
                }
                if (self.rows[i]['elements'][j].hasAttribute('cy')) {
                    const new_y1 = Number(self.rows[i]['elements'][j].getAttribute('cy')) - self.Y_SPACING;
                    self.rows[i]['elements'][j].setAttribute('cy', new_y1.toString());
                }
                if (self.rows[i]['elements'][j].hasAttribute('y')) {
                    const new_y1 = Number(self.rows[i]['elements'][j].getAttribute('y')) - self.Y_SPACING;
                    self.rows[i]['elements'][j].setAttribute('y', new_y1.toString());
                }
            }
        }

        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.setVisibleCommits();
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

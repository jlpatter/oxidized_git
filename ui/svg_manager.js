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
    CIRCLE_RADIUS = 5;
    TEXT_Y_OFFSET = 5;  // If updating, be sure to update on the back-end as well!
    RECT_HEIGHT = 18;  // If updating, be sure to update on the back-end as well!
    RECT_Y_OFFSET = -(this.RECT_HEIGHT / 2);  // If updating, be sure to update on the back-end as well!
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

        if (!commitsInfo['clear_entire_old_graph'] && commitsInfo['deleted_shas'].length > 0) {
            self.removeRows(commitsInfo['deleted_shas']);
        }

        // Remove all lines and branch labels as they will be re-added later.
        for (let i = 0; i < self.rows.length; i++) {
            self.rows[i]['lines'] = [];
            self.rows[i]['branches'] = [];
            self.rows[i]['x'] = 0;
        }

        const newRows = [];
        for (let i = 0; i < commitsInfo['created_commit_info_list'].length; i++) {
            const commit = commitsInfo['created_commit_info_list'][i];

            const summaryTxtElement = self.makeSVG('text', {'fill': 'white'});
            summaryTxtElement.textContent = commit['summary'];

            const backRectElement = self.makeSVG('rect', {'class': 'svg-hoverable-row', 'height': self.RECT_HEIGHT, 'style': 'fill:white;fill-opacity:0.1;'});
            backRectElement.onclick = self.getClickFunction(commit['sha']);
            backRectElement.ondblclick = self.getDblClickFunction(commit['sha']);
            backRectElement.oncontextmenu = self.getContextFunction(commit['sha']);

            const row = {
                'sha': commit['sha'],
                'x': 0,
                'childShas': commit['child_shas'],
                'parentShas': commit['parent_shas'],
                'circle': self.makeSVG('circle', {'r': self.CIRCLE_RADIUS, 'stroke-width': 1}),
                'summaryTxt': summaryTxtElement,
                'backRect': backRectElement,
                'lines': [],
                'branches': [],
            };

            newRows.push(row);
        }
        self.rows = newRows.concat(self.rows);

        const occupiedTable = self.getOccupiedTable();
        let maxWidth = self.setPositions(occupiedTable, singleCharWidth);

        maxWidth = self.addBranchLabels(commitsInfo['branch_draw_properties'], singleCharWidth, maxWidth);

        self.setVisibleCommits();
        self.commitTableSVG.setAttribute('width', maxWidth.toString());
        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
    }

    getOccupiedTable() {
        const self = this;

        let occupiedTable = [];

        for (let y = 0; y < self.rows.length; y++) {
            // Set the current node position as occupied (or find a position that's unoccupied and occupy it).
            if (y < occupiedTable.length) {
                while (occupiedTable[y].indexOf(self.rows[y]['x']) !== -1) {
                    self.rows[y]['x']++;
                }
                occupiedTable[y].push(self.rows[y]['x']);
            } else if (y === occupiedTable.length) {
                occupiedTable.push([self.rows[y]['x']]);
            } else {
                console.error('y was bigger than the next position in the occupied table.');
            }

            // Set the space of the line from the current node to its parents as occupied.
            const parentIndexes = self.getIndexesFromSHAs(self.rows[y]['parentShas']);
            parentIndexes.forEach((parentY) => {
                for (let lineY = y + 1; lineY < parentY; lineY++) {
                    if (lineY < occupiedTable.length) {
                        occupiedTable[lineY].push(self.rows[y]['x']);
                    } else if (lineY === occupiedTable.length) {
                        occupiedTable.push([self.rows[y]['x']]);
                    } else {
                        console.error('y was bigger than the next position in the occupied table.');
                    }
                }
            });

            // Set curved lines to occupy their space (note this has to be done with children since their x value is already set unlike the parents)
            const childIndexes = self.getIndexesFromSHAs(self.rows[y]['childShas']);
            childIndexes.forEach((childY) => {
                if (self.rows[y]['x'] < self.rows[childY]['x']) {
                    occupiedTable[y].push(self.rows[childY]['x']);
                } else if (self.rows[y]['x'] > self.rows[childY]['x']) {
                    occupiedTable[childY].push(self.rows[y]['x']);
                }
            });
        }

        return occupiedTable;
    }

    setPositions(occupiedTable, singleCharWidth) {
        const self = this;
        let maxWidth = 0;
        // Since adding or removing even a single row causes all y values to need to be updated, might as well update them all here...
        for (let y = 0; y < self.rows.length; y++) {
            const pixelX = self.rows[y]['x'] * self.X_SPACING + self.X_OFFSET;
            const pixelY = y * self.Y_SPACING + self.Y_OFFSET;
            const color = self.getColorString(self.rows[y]['x']);

            // Create lines from children to current row.
            const childIndexes = self.getIndexesFromSHAs(self.rows[y]['childShas']);
            childIndexes.forEach((childY) => {
                const childPixelX = self.rows[childY]['x'] * self.X_SPACING + self.X_OFFSET;
                const beforeY = y - 1;
                const beforePixelY = beforeY * self.Y_SPACING + self.Y_OFFSET;

                for (let i = childY; i < beforeY; i++) {
                    const topPixelY = i * self.Y_SPACING + self.Y_OFFSET;
                    const bottomPixelY = (i + 1) * self.Y_SPACING + self.Y_OFFSET;

                    const styleString = 'stroke:' + self.getColorString(self.rows[childY]['x']) + ';stroke-width:' + self.LINE_STROKE_WIDTH + ';';
                    const lineElement = self.makeSVG('line', {'x1': childPixelX, 'y1': topPixelY, 'x2': childPixelX, 'y2': bottomPixelY, 'style': styleString});
                    self.rows[i + 1]['lines'].push({'target-sha': self.rows[childY]['sha'], 'element': lineElement});
                }

                let styleString = 'stroke:';
                if (self.rows[childY]['x'] >= self.rows[y]['x']) {
                    // Sets the color for "branching" lines and straight lines
                    styleString += self.getColorString(self.rows[childY]['x']);
                } else {
                    // Sets the color for "merging" lines
                    styleString += self.getColorString(self.rows[y]['x']);
                }
                styleString += ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH + ';';

                if (childPixelX === pixelX) {
                    const lineElement = self.makeSVG('line', {'x1': childPixelX, 'y1': beforePixelY, 'x2': pixelX, 'y2': pixelY, 'style': styleString});
                    self.rows[y]['lines'].push({'target-sha': self.rows[childY]['sha'], 'element': lineElement});
                } else {
                    let dString = 'M ' + childPixelX + ' ' + beforePixelY + ' C ';
                    if (childPixelX < pixelX) {
                        const startControlPointX = childPixelX + self.X_SPACING * 3 / 4;
                        const endControlPointY = pixelY - self.Y_SPACING * 3 / 4;
                        dString += startControlPointX + ' ' + beforePixelY + ', ' + pixelX + ' ' + endControlPointY;
                    } else {
                        let startControlPointY = beforePixelY + self.Y_SPACING * 3 / 4;
                        let endControlPointX = pixelX + self.X_SPACING * 3 / 4;
                        dString += childPixelX + ' ' + startControlPointY + ', ' + endControlPointX + ' ' + pixelY;
                    }
                    dString += ', ' + pixelX + ' ' + pixelY;

                    const pathElement = self.makeSVG('path', {'d': dString, 'style': styleString});
                    self.rows[y]['lines'].push({'target-sha': self.rows[childY]['sha'], 'element': pathElement});
                }
            });

            // Set circle attributes
            self.rows[y]['circle'].setAttribute('cx', pixelX.toString());
            self.rows[y]['circle'].setAttribute('cy', pixelY.toString());
            self.rows[y]['circle'].setAttribute('stroke', color);
            self.rows[y]['circle'].setAttribute('fill', color);

            // Set summaryTxt attributes
            const largestOccupiedX = Math.max(...occupiedTable[y], 0);
            const summaryTxtPixelX = (largestOccupiedX + 1) * self.X_SPACING + self.X_OFFSET;
            self.rows[y]['summaryTxt'].setAttribute('x', summaryTxtPixelX.toString());
            self.rows[y]['summaryTxt'].setAttribute('y', (pixelY + self.TEXT_Y_OFFSET).toString());

            // Set backRect attributes
            self.rows[y]['backRect'].setAttribute('x', pixelX.toString());
            self.rows[y]['backRect'].setAttribute('y', (pixelY + self.RECT_Y_OFFSET).toString());
            const backRectWidth = summaryTxtPixelX + self.rows[y]['summaryTxt'].textContent.length * singleCharWidth;
            self.rows[y]['backRect'].setAttribute('width', backRectWidth.toString());
            maxWidth = Math.max(backRectWidth, maxWidth);
        }
        return maxWidth;
    }

    getIndexesFromSHAs(SHAs) {
        const self = this;
        return SHAs.map(function(SHA) {
            return self.rows.findIndex(function(row) {
                return row['sha'] === SHA;
            });
        }).filter(function(index) { return index !== -1; });
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
                backRectElem.setAttribute('width', (currentPixelX + summaryTxtElem.textContent.length * singleCharWidth).toString());
                maxWidth = Math.max(maxWidth, currentPixelX + summaryTxtElem.textContent.length * singleCharWidth);
            }
        }
        return maxWidth;
    }

    getColorString(x) {
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

                let parentIndexes = self.getIndexesFromSHAs(tempParentShas);

                // Remove the line coming from the parent commit(s)
                for (let i = 0; i < parentIndexes.length; i++) {
                    const lineIndexToRemove = self.rows[parentIndexes[i]]['lines'].findIndex(function(line) {
                        return line['target-sha'] === sha;
                    });
                    if (lineIndexToRemove !== -1) {
                        self.rows[parentIndexes[i]]['lines'].splice(lineIndexToRemove, 1);
                    } else {
                        console.error('Line not found from parent going to deleted child!');
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

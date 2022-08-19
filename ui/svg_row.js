/**
 * A row in the svg table.
 */
import {writeText} from '@tauri-apps/api/clipboard';

export class SVGRow {
    X_OFFSET = 20;
    Y_OFFSET = 20;
    X_SPACING = 20;
    Y_SPACING = 30;
    TEXT_Y_ALIGNMENT = 6;
    CIRCLE_RADIUS = 10;
    RECT_Y_OFFSET = -12;
    RECT_HEIGHT = 24;
    BRANCH_TEXT_SPACING = 5;

    /**
     * Construct the svg row
     * @param {string} sha
     * @param {string} summary
     * @param {Array<string>} branchesAndTags
     * @param {Array<string>} parentShas
     * @param {Array<string>} childrenShas
     * @param {int} x
     * @param {int} y
     */
    constructor(sha, summary, branchesAndTags, parentShas, childrenShas, x, y) {
        this.sha = sha;
        this.summary = summary;
        this.branchesAndTags = branchesAndTags;
        this.parentShas = parentShas;
        this.childrenShas = childrenShas;
        this.x = x;
        this.y = y;
        this.width = 0;
    }

    /**
     * Gets the SVGRow parents
     * @param {Array<SVGRow>} array
     * @return {Array<SVGRow>}
     */
    getParentSVGRows(array) {
        const self = this;
        if (self.parentShas.length === 0) {
            return [];
        }
        const parentSVGRows = [];
        for (let i = 0; i < self.parentShas.length; i++) {
            for (let j = 0; j < array.length; j++) {
                if (self.parentShas[i] === array[j].sha) {
                    parentSVGRows.push(array[j]);
                    break;
                }
            }
        }
        return parentSVGRows;
    }

    /**
     * Gets the SVGRow children
     * @param {Array<SVGRow>} array
     * @return {Array<SVGRow>}
     */
    getChildSVGRows(array) {
        const self = this;
        if (self.childrenShas.length === 0) {
            return [];
        }
        const childSVGRows = [];
        for (let i = 0; i < self.childrenShas.length; i++) {
            for (let j = 0; j < array.length; j++) {
                if (self.childrenShas[i] === array[j].sha) {
                    childSVGRows.push(array[j]);
                    break;
                }
            }
        }
        return childSVGRows;
    }

    /**
     * Draw each of the components of the svg row.
     * @param {jQuery} $commitTableSVG
     * @param {Array<SVGRow>} parentSVGRows
     * @param {Array<SVGRow>} childSVGRows
     * @param {Object<number, Object<number, boolean>>} mainTable
     */
    draw($commitTableSVG, parentSVGRows, childSVGRows, mainTable) {
        const self = this;

        // Set the current node position as occupied (or find a position that's unoccupied and occupy it).
        if (!(self.y in mainTable)) {
            mainTable[self.y] = {};
            mainTable[self.y][self.x] = true;
        } else if (!(self.x in mainTable[self.y])) {
            mainTable[self.y][self.x] = true;
        } else if (mainTable[self.y][self.x] === true) {
            let foundEmpty = false;
            while (!foundEmpty) {
                self.x++;
                if (!(self.x in mainTable[self.y])) {
                    foundEmpty = true;
                    mainTable[self.y][self.x] = true;
                }
            }
        }

        // Set the space of the line from the current node to its parents as occupied.
        const pixelX = self.x * self.X_SPACING + self.X_OFFSET;
        const pixelY = self.y * self.Y_SPACING + self.Y_OFFSET;
        const color = self.getColor(self.x);
        if (parentSVGRows.length > 0) {
            for (let i = 0; i < parentSVGRows.length; i++) {
                // ParentSVGRows are lower on the graph (with a higher y value).
                for (let j = self.y + 1; j < parentSVGRows[i].y; j++) {
                    if (!(j in mainTable)) {
                        mainTable[j] = {};
                        mainTable[j][self.x] = true;
                    } else if (!(self.x in mainTable[j])) {
                        mainTable[j][self.x] = true;
                    }
                }
            }
        }

        // Draw the lines from the current node to its children.
        if (childSVGRows.length > 0) {
            for (let i = 0; i < childSVGRows.length; i++) {
                const childPixelX = childSVGRows[i].x * self.X_SPACING + self.X_OFFSET;
                const childPixelY = childSVGRows[i].y * self.Y_SPACING + self.Y_OFFSET;
                const beforePixelY = (self.y - 1) * self.Y_SPACING + self.Y_OFFSET;
                const svgLine = self.makeSVG('line', {x1: childPixelX, y1: childPixelY, x2: childPixelX, y2: beforePixelY, style: 'stroke:' + self.getColor(childSVGRows[i].x) + ';stroke-width:4'});
                const angledSVGLine = self.makeSVG('line', {x1: childPixelX, y1: beforePixelY, x2: pixelX, y2: pixelY, style: 'stroke:' + self.getColor(childSVGRows[i].x) + ';stroke-width:4'});
                $commitTableSVG.append(svgLine);
                $commitTableSVG.append(angledSVGLine);
            }
        }

        // Now draw the node.
        const svgCircle = self.makeSVG('circle', {'cx': pixelX, 'cy': pixelY, 'r': self.CIRCLE_RADIUS, 'stroke': color, 'stroke-width': 1, 'fill': color});
        $commitTableSVG.append(svgCircle);

        // Draw the branch text.
        const occupiedRowNums = Object.keys(mainTable[self.y]);
        let largestXValue = 0;
        for (let i = 0; i < occupiedRowNums.length; i++) {
            largestXValue = Math.max(largestXValue, Number(occupiedRowNums[i]));
        }
        let currentX = (largestXValue + 1) * self.X_SPACING + self.X_OFFSET;
        const contextFunction = self.getContextFunction();
        for (let i = 0; i < self.branchesAndTags.length; i++) {
            const svgTextElem = self.makeSVG('text', {x: currentX, y: pixelY + self.TEXT_Y_ALIGNMENT, fill: 'white'});
            svgTextElem.textContent = self.branchesAndTags[i]['branch_name'];
            svgTextElem.oncontextmenu = contextFunction;
            const branchRectId = 'branch_rect_' + i + '_' + self.sha;
            let branchRectColor = 'yellow';
            if (self.branchesAndTags[i]['branch_type'] === 'local') {
                branchRectColor = 'red';
            } else if (self.branchesAndTags[i]['branch_type'] === 'remote') {
                branchRectColor = 'green';
            } else if (self.branchesAndTags[i]['branch_type'] === 'tag') {
                branchRectColor = 'grey';
            }
            const svgRectElem = self.makeSVG('rect', {id: branchRectId, x: currentX - 5, y: pixelY + self.RECT_Y_OFFSET, rx: 10, ry: 10, width: 0, height: self.RECT_HEIGHT, style: 'fill:' + branchRectColor + ';fill-opacity:0.5;'});
            $commitTableSVG.append(svgRectElem);
            $commitTableSVG.append(svgTextElem);
            const box_width = svgTextElem.getBBox().width + 10;
            // TODO: This may be causing slowdown. Use document.getElementById('#id');
            $('#' + branchRectId).attr('width', box_width);
            currentX += box_width + self.BRANCH_TEXT_SPACING;
        }

        // Draw the summary text.
        const entryElem = self.makeSVG('text', {x: currentX, y: pixelY + self.TEXT_Y_ALIGNMENT, fill: 'white'});
        entryElem.textContent = self.summary;
        entryElem.oncontextmenu = contextFunction;
        $commitTableSVG.append(entryElem);
        self.width = currentX + entryElem.getBBox().width;

        const rectElem = self.makeSVG('rect', {class: 'backgroundRect', x: pixelX, y: pixelY + self.RECT_Y_OFFSET, width: self.width, height: self.RECT_HEIGHT, style: 'fill:white;fill-opacity:0.1;'});
        rectElem.oncontextmenu = contextFunction;
        $commitTableSVG.append(rectElem);
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

    /**
     * Gets the color of the row based on the indent
     * @param {number} xValue
     * @return {string}
     */
    getColor(xValue) {
        const colorNum = xValue % 4;
        if (colorNum === 0) {
            return '\#00CC19';
        } else if (colorNum === 1) {
            return '\#0198A6';
        } else if (colorNum === 2) {
            return '\#FF7800';
        } else {
            return '\#FF0D00';
        }
    }

    /**
     * Gets the function to be called by oncontextmenu
     * @return {(function(*): void)|*}
     */
    getContextFunction() {
        const self = this;
        return function(event) {
            event.preventDefault();
            const $contextMenu = $('#contextMenu');
            $contextMenu.empty();
            $contextMenu.css('left', event.pageX + 'px');
            $contextMenu.css('top', event.pageY + 'px');

            const $mergeBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-arrows-angle-contract"></i> Merge</button>');
            $mergeBtn.click(function() {
                // TODO: Implement stuff here
            });
            $contextMenu.append($mergeBtn);

            const $cherrypickBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-bullseye"></i> Cherrypick Commit</button>');
            $cherrypickBtn.click(function() {
                // TODO: Implement stuff here
            });
            $contextMenu.append($cherrypickBtn);

            const $copyShaBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-clipboard"></i> Copy SHA</button>');
            $copyShaBtn.click(function() {
                writeText(self.sha).then();
            });
            $contextMenu.append($copyShaBtn);

            const $softResetBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-arrow-clockwise"></i> Soft Reset to Here</button>');
            $softResetBtn.click(function() {
                // TODO: Implement stuff here
            });
            $contextMenu.append($softResetBtn);

            const $mixedResetBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-arrow-clockwise"></i> Mixed Reset to Here</button>');
            $mixedResetBtn.click(function() {
                // TODO: Implement stuff here
            });
            $contextMenu.append($mixedResetBtn);

            const $hardResetBtn = $('<button type="button" class="btn btn-outline-light btn-sm rounded-0 cm-item"><i class="bi bi-arrow-clockwise"></i> Hard Reset to Here</button>');
            $hardResetBtn.click(function() {
                // TODO: Implement stuff here
            });
            $contextMenu.append($hardResetBtn);

            $contextMenu.show();
        };
    }
}

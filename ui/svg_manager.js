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
    LINE_STROKE_WIDTH = 2;  // If changing, be sure to update on backend-end too
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
            self.removeRows(commitsInfo['deleted_shas'], singleCharWidth);
        }

        const newRows = [];
        let maxWidth = Number(self.commitTableSVG.getAttribute('width'));
        for (let i = 0; i < commitsInfo['svg_row_draw_properties'].length; i++) {
            const commit = commitsInfo['svg_row_draw_properties'][i];
            const elements = commit['elements'];
            let row = {'sha': commit['sha'], 'childShas': commit['child-shas'], 'parentShas': commit['parent-shas'], 'pixel_y': commit['pixel_y'], 'lines': [], 'branches': []};
            for (const childLine of elements['child_lines']) {
                const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                if (childLine['row-y'] < i) {
                    newRows[childLine['row-y']]['lines'].push({'element': line, 'target-sha': childLine['target-sha']});
                } else if (childLine['row-y'] === i) {
                    row['lines'].push({'element': line, 'target-sha': childLine['target-sha']});
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

        if (newRows.length > 0) {
            self.addRows(newRows, singleCharWidth);
        }

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

    addRows(newRows, singleCharWidth) {
        const self = this,
            startIndex = newRows.length,
            amountToMove = self.Y_SPACING * newRows.length;

        let pixel_y = amountToMove + self.Y_OFFSET;
        self.rows = newRows.concat(self.rows);

        for (let i = startIndex; i < self.rows.length; i++) {
            self.rows[i]['pixel_y'] = pixel_y;
            self.moveYAttributes(self.rows[i]['lines'].map(function(line) {return line['element'];}), amountToMove);
            self.moveYAttributes(self.rows[i]['branches'], amountToMove);
            self.moveYAttributes([self.rows[i]['circle'], self.rows[i]['summaryTxt'], self.rows[i]['backRect']], amountToMove);
            pixel_y += self.Y_SPACING;
        }

        for (let i = startIndex; i < self.rows.length; i++) {
            for (let j = 0; j < newRows.length; j++) {
                const childWasJustAdded = self.rows[i]['childShas'].findIndex(function(childSha) {
                    return childSha === self.rows[j]['sha'];
                }) !== -1;
                if (childWasJustAdded) {
                    self.addChildLines(self.rows[i], i, singleCharWidth);
                    break;
                }
            }
        }

        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.setVisibleCommits();
    }

    addChildLines(row, rowIndex, singleCharWidth) {
        const self = this;
        row['childShas'].forEach((childSha) => {
            let childRow;
            let childRowY;
            for (let i = 0; i < self.rows.length; i++) {
                if (self.rows[i]['sha'] === childSha) {
                    childRow = self.rows[i];
                    childRowY = i;
                    break;
                }
            }

            const childPixelX = Number(childRow['circle'].getAttribute('cx'));
            const childPixelY = Number(childRow['circle'].getAttribute('cy'));
            const childRowX = (childPixelX - self.Y_OFFSET) / self.Y_SPACING;
            const beforeY = rowIndex - 1;
            const beforePixelY = beforeY * self.Y_SPACING + self.Y_OFFSET;

            if (beforePixelY !== childPixelY) {
                for (let i = childRowY; i < beforeY; i++) {
                    const topPixelY = i * self.Y_SPACING + self.Y_OFFSET;
                    const bottomPixelY = (i + 1) * self.Y_SPACING + self.Y_OFFSET;
                    const styleString = 'stroke:' + self.get_color_string(childRowX) + ';stroke-width:' + self.LINE_STROKE_WIDTH.toString() + ';';
                    const lineElement = self.makeSVG('line', {'x1': childPixelX, 'y1': topPixelY, 'x2': childPixelX, 'y2': bottomPixelY, 'style': styleString});
                    const line = {'element': lineElement, 'target-sha': childSha};

                    // Note: Lines just need to be moved, not updated.
                    self.moveXAttributes(self.rows[i + 1]['lines'].map(function(line) { return line['element']; }), self.X_SPACING);
                    self.moveXAttributes(self.rows[i + 1]['branches'], self.X_SPACING);
                    self.moveXAttributes([self.rows[i + 1]['summaryTxt']], self.X_SPACING);
                    if (Number(self.rows[i + 1]['circle'].getAttribute('cx')) >= childPixelX) {
                        self.moveXAttributes([self.rows[i + 1]['circle'], self.rows[i + 1]['backRect']], self.X_SPACING);
                    }
                    const newWidth = Number(self.rows[i + 1]['summaryTxt'].getAttribute('x')) + self.rows[i + 1]['summaryTxt'].textContent.length * singleCharWidth;
                    self.rows[i + 1]['backRect'].setAttribute('width', newWidth.toString());

                    self.rows[i + 1]['lines'].push(line);
                }
            }

            const rowPixelX = Number(row['circle'].getAttribute('cx'));
            const rowPixelY = Number(row['circle'].getAttribute('cy'));
            const rowX = (rowPixelX - self.Y_OFFSET) / self.Y_SPACING;
            let styleString = 'stroke:';
            if (childRowX >= rowX) {
                // Sets the color for "branching" lines and straight lines
                styleString += self.get_color_string(childRowX);
            } else {
                // Sets the color for "merging" lines
                styleString += self.get_color_string(rowX);
            }
            styleString += ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH.toString() + ';';
            if (childPixelX === rowPixelX) {
                const lineElement = self.makeSVG('line', {'x1': childPixelX, 'y1': beforePixelY, 'x2': rowPixelX, 'y2': rowPixelY, 'style': styleString});
                const line = {'element': lineElement, 'target-sha': childSha};

                self.updateLines(row['lines'], childPixelX);
                row['lines'].push(line);
            } else {
                let dString = 'M ' + childPixelX + ' ' + beforePixelY + ' C ';
                if (childPixelX < rowPixelX) {
                    const startControlPointX = childPixelX + self.X_SPACING * 3 / 4;
                    const endControlPointY = rowPixelY - self.Y_SPACING * 3 / 4;
                    dString += startControlPointX + ' ' + beforePixelY + ', ' + rowPixelX + ' ' + endControlPointY + ', ';
                } else {
                    let startControlPointY = beforePixelY + self.Y_SPACING * 3 / 4;
                    let endControlPointX = rowPixelX + self.X_SPACING * 3 / 4;
                    dString += childPixelX + ' ' + startControlPointY + ', ' + endControlPointX + ' ' + rowPixelY + ', ';
                }
                dString += rowPixelX + ' ' + rowPixelY;

                const pathElement = self.makeSVG('path', {'d': dString, 'style': styleString});
                const path = {'element': pathElement, 'target-sha': childSha};

                let rightPixelX;
                if (rowPixelX > childPixelX) {
                    rightPixelX = rowPixelX;
                } else {
                    rightPixelX = childPixelX;
                }

                self.moveXAttributes(row['branches'], self.X_SPACING);
                self.moveXAttributes([row['summaryTxt']], self.X_SPACING);
                if (Number(row['circle'].getAttribute('cx')) >= rightPixelX) {
                    self.moveXAttributes([row['circle'], row['backRect']], self.X_SPACING);
                }
                const newWidth = Number(row['summaryTxt'].getAttribute('x')) + row['summaryTxt'].textContent.length * singleCharWidth;
                row['backRect'].setAttribute('width', newWidth.toString());

                self.updateLines(row['lines'], rightPixelX);
                row['lines'].push(path);
            }
        });
    }

    // If changing, be sure to update on backend-end too
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

    get_color_string_pixel(pixelX) {
        const self = this;
        return self.get_color_string((pixelX - self.X_OFFSET) / self.X_SPACING);
    }

    removeRows(shas, singleCharWidth) {
        const self = this;

        shas.forEach((sha) => {
            const startIndex = self.rows.findIndex(function(row) {
                return row['sha'] === sha;
            });

            if (startIndex === -1) {
                console.error("Couldn't find row to remove from graph!");
            } else {
                let pixelY = self.rows[startIndex]['pixel_y'];

                let parentIndexes = self.rows[startIndex]['parentShas'].map(function(parentSha) {
                    return self.rows.findIndex(function(row) {
                        return row['sha'] === parentSha;
                    });
                });

                self.rows.splice(startIndex, 1);
                for (let i = 0; i < parentIndexes.length; i++) {
                    parentIndexes[i]--;
                }

                // Remove the line coming from the parent commit(s)
                for (let i = 0; i < parentIndexes.length; i++) {
                    const lineIndexToRemove = self.rows[parentIndexes[i]]['lines'].findIndex(function(line) {
                        return line['target-sha'] === sha;
                    });
                    if (lineIndexToRemove !== -1) {
                        self.rows[parentIndexes[i]]['lines'].splice(lineIndexToRemove, 1);
                    }
                }

                for (let j = startIndex; j < self.rows.length; j++) {
                    self.rows[j]['pixel_y'] = pixelY;
                    self.moveYAttributes(self.rows[j]['lines'].map(function(line) {return line['element'];}), -self.Y_SPACING);
                    self.moveYAttributes(self.rows[j]['branches'], -self.Y_SPACING);
                    self.moveYAttributes([self.rows[j]['circle'], self.rows[j]['summaryTxt'], self.rows[j]['backRect']], -self.Y_SPACING);
                    pixelY += self.Y_SPACING;
                }
            }
        });

        self.commitTableSVG.setAttribute('height', ((self.rows.length + 1) * self.Y_SPACING).toString());
        self.setVisibleCommits();
    }

    updateLines(lines, newLinePixelX) {
        const self = this;
        for (let i = 0; i < lines.length; i++) {
            if (lines[i]['element'].tagName === "PATH") {
                // This assumes 'd' is structured like the following: "M x1 y1 C x2 y2, x3 y3, x4 y4"
                const oldD = lines[i]['element'].getAttribute('d').split(', ');
                const firstElemSplit = oldD.shift().split(' C ');
                const firstPair = firstElemSplit[0].slice(2).split(' ');
                const fourthPair = oldD[1].split(' ');

                const childPixelX = newLinePixelX;
                const rowPixelX = Number(fourthPair[0]);
                const beforePixelY = Number(firstPair[1]);
                const rowPixelY = Number(fourthPair[1]);

                let newD = 'M ' + childPixelX + ' ' + firstPair[1] + ' C ';
                if (childPixelX < rowPixelX) {
                    const startControlPointX = childPixelX + self.X_SPACING * 3 / 4;
                    const endControlPointY = rowPixelY - self.Y_SPACING * 3 / 4;
                    newD += startControlPointX + ' ' + beforePixelY + ', ' + rowPixelX + ' ' + endControlPointY + ', ';
                } else {
                    let startControlPointY = beforePixelY + self.Y_SPACING * 3 / 4;
                    let endControlPointX = rowPixelX + self.X_SPACING * 3 / 4;
                    newD += childPixelX + ' ' + startControlPointY + ', ' + endControlPointX + ' ' + rowPixelY + ', ';
                }
                newD += rowPixelX + ' ' + rowPixelY;

                const childRowX = (childPixelX - self.X_OFFSET) / self.X_SPACING;
                const rowX = (rowPixelX - self.X_OFFSET) / self.X_SPACING;
                let styleString = 'stroke:';
                if (childRowX >= rowX) {
                    // Sets the color for "branching" lines and straight lines
                    styleString += self.get_color_string(childRowX);
                } else {
                    // Sets the color for "merging" lines
                    styleString += self.get_color_string(rowX);
                }
                styleString += ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH.toString() + ';';

                lines[i]['element'].setAttribute('d', newD);
                lines[i]['element'].setAttribute('style', styleString);
            } else if (lines[i]['element'].tagName === "LINE") {
                const childPixelX = newLinePixelX;
                const rowPixelX = Number(lines[i]['element'].getAttribute('x2'));
                const beforePixelY = Number(lines[i]['element'].getAttribute('y1'));
                const rowPixelY = Number(lines[i]['element'].getAttribute('y2'));

                let dString = 'M ' + childPixelX + ' ' + beforePixelY + ' C ';
                if (childPixelX < rowPixelX) {
                    const startControlPointX = childPixelX + self.X_SPACING * 3 / 4;
                    const endControlPointY = rowPixelY - self.Y_SPACING * 3 / 4;
                    dString += startControlPointX + ' ' + beforePixelY + ', ' + rowPixelX + ' ' + endControlPointY + ', ';
                } else {
                    let startControlPointY = beforePixelY + self.Y_SPACING * 3 / 4;
                    let endControlPointX = rowPixelX + self.X_SPACING * 3 / 4;
                    dString += childPixelX + ' ' + startControlPointY + ', ' + endControlPointX + ' ' + rowPixelY + ', ';
                }
                dString += rowPixelX + ' ' + rowPixelY;

                const childRowX = (childPixelX - self.X_OFFSET) / self.X_SPACING;
                const rowX = (rowPixelX - self.X_OFFSET) / self.X_SPACING;
                let styleString = 'stroke:';
                if (childRowX >= rowX) {
                    // Sets the color for "branching" lines and straight lines
                    styleString += self.get_color_string(childRowX);
                } else {
                    // Sets the color for "merging" lines
                    styleString += self.get_color_string(rowX);
                }
                styleString += ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH.toString() + ';';

                lines[i]['element'] = self.makeSVG('path', {'d': dString, 'style': styleString});
            }
        }
    }

    moveXAttributes(elements, amountToMove) {
        const self = this;
        for (let j = 0; j < elements.length; j++) {
            if (elements[j].hasAttribute('x1')) {
                const new_x1 = Number(elements[j].getAttribute('x1')) + amountToMove;
                elements[j].setAttribute('x1', new_x1.toString());
                elements[j].setAttribute('style', 'stroke:' + self.get_color_string_pixel(new_x1) + ';stroke-width:' + self.LINE_STROKE_WIDTH + ';');
            }
            if (elements[j].hasAttribute('x2')) {
                const new_x2 = Number(elements[j].getAttribute('x2')) + amountToMove;
                elements[j].setAttribute('x2', new_x2.toString());
                elements[j].setAttribute('style', 'stroke:' + self.get_color_string_pixel(new_x2) + ';stroke-width:' + self.LINE_STROKE_WIDTH + ';');
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
                    (Number(firstPair[0]) + amountToMove).toString() + ' ' +
                    firstPair[1] +
                    ' C ' +
                    (Number(secondPair[0]) + amountToMove).toString() + ' ' +
                    secondPair[1] + ', ' +
                    (Number(thirdPair[0]) + amountToMove).toString() + ' ' +
                    thirdPair[1] + ', ' +
                    (Number(fourthPair[0]) + amountToMove).toString() + ' ' +
                    fourthPair[1];
                elements[j].setAttribute('d', newD);
                if (Number(firstPair[0]) >= Number(fourthPair[0])) {
                    elements[j].setAttribute('style', 'stroke:' + self.get_color_string_pixel(Number(firstPair[0])) + ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH + ';');
                } else {
                    elements[j].setAttribute('style', 'stroke:' + self.get_color_string_pixel(Number(fourthPair[0])) + ';fill:transparent;stroke-width:' + self.LINE_STROKE_WIDTH + ';');
                }
            }
            if (elements[j].hasAttribute('cx')) {
                const new_cx = Number(elements[j].getAttribute('cx')) + amountToMove;
                elements[j].setAttribute('cx', new_cx.toString());
                elements[j].setAttribute('stroke', self.get_color_string_pixel(new_cx));
                elements[j].setAttribute('fill', self.get_color_string_pixel(new_cx));
            }
            if (elements[j].hasAttribute('x')) {
                const new_x = Number(elements[j].getAttribute('x')) + amountToMove;
                elements[j].setAttribute('x', new_x.toString());
            }
        }
    }

    moveYAttributes(elements, amountToMove) {
        for (let j = 0; j < elements.length; j++) {
            if (elements[j].hasAttribute('y1')) {
                const new_y1 = Number(elements[j].getAttribute('y1')) + amountToMove;
                elements[j].setAttribute('y1', new_y1.toString());
            }
            if (elements[j].hasAttribute('y2')) {
                const new_y2 = Number(elements[j].getAttribute('y2')) + amountToMove;
                elements[j].setAttribute('y2', new_y2.toString());
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
                const new_cy = Number(elements[j].getAttribute('cy')) + amountToMove;
                elements[j].setAttribute('cy', new_cy.toString());
            }
            if (elements[j].hasAttribute('y')) {
                const new_y = Number(elements[j].getAttribute('y')) + amountToMove;
                elements[j].setAttribute('y', new_y.toString());
            }
        }
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

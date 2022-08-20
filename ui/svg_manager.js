import {writeText} from "@tauri-apps/api/clipboard";

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
        // TODO: Refactor this to use 1 array with a top and bottom index.
        this.commitsAbove = [];
        this.commitsVisible = [];
        this.commitsBelow = [];
        this.oldRenderingAreaTop = 0;
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

        const textSizeTest = self.makeSVG('text', {x: 0, y: 0, fill: 'white'});
        textSizeTest.textContent = 'A';
        self.commitTableSVG.appendChild(textSizeTest);
        const singleCharWidth = textSizeTest.getBBox().width;

        self.commitTableSVG.setAttribute('height', (self.repoInfo.length * 30).toString());

        self.commitsAbove = [];
        self.commitsVisible = [];
        self.commitsBelow = [];
        const renderingAreaTop = self.oldRenderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN;
        const renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;

        const contextFunction = self.getContextFunction();
        let maxWidth = 0;
        for (const commit of self.repoInfo) {
            let row_elements = {'pixel_y': commit[1]['attrs']['cy'], 'rows': []};
            for (const childLine of commit[0]) {
                const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                row_elements['rows'].push(line);
            }
            const circle = self.makeSVG(commit[1]['tag'], commit[1]['attrs']);
            row_elements['rows'].push(circle);

            let currentX = -1;
            for (const branch_or_tags of commit[2]) {
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
                row_elements['rows'].push(rectElem);
                row_elements['rows'].push(txtElem);
                currentX += box_width + self.BRANCH_TEXT_SPACING;
            }

            if (currentX === -1) {
                currentX = (commit[3]['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
            }
            commit[3]['attrs']['x'] = currentX;
            const summaryTxt = self.makeSVG(commit[3]['tag'], commit[3]['attrs']);
            summaryTxt.textContent = commit[3]['textContent'];
            row_elements['rows'].push(summaryTxt);

            let width = currentX + commit[3]['textContent'].length * singleCharWidth;
            commit[4]['attrs']['width'] = width;
            const backRect = self.makeSVG(commit[4]['tag'], commit[4]['attrs']);
            backRect.oncontextmenu = contextFunction;
            row_elements['rows'].push(backRect);

            if (row_elements['pixel_y'] < renderingAreaTop) {
                self.commitsAbove.push(row_elements);
            } else if (row_elements['pixel_y'] > renderingAreaBottom) {
                self.commitsBelow.push(row_elements);
            } else {
                self.commitsVisible.push(row_elements);
            }
            maxWidth = Math.max(maxWidth, width);
        }

        self.renderVisibleCommits();
        self.commitTableSVG.setAttribute('width', maxWidth.toString());
    }

    renderVisibleCommits() {
        const self = this;

        let df = document.createDocumentFragment();
        for (const row of self.commitsVisible) {
            for (const element of row['rows']) {
                df.appendChild(element);
            }
        }

        self.commitTableSVG.innerHTML = '';

        self.commitTableSVG.appendChild(df);
    }

    setScrollEvent() {
        const self = this;
        self.commitColumn.addEventListener('scroll', () => {
            let renderingAreaTop = self.commitColumn.scrollTop - self.SCROLL_RENDERING_MARGIN;
            let renderingAreaBottom = self.commitColumn.scrollTop + self.commitColumn.clientHeight + self.SCROLL_RENDERING_MARGIN;
            if (renderingAreaTop < self.oldRenderingAreaTop) {
                // Scrolling Up
                // Remove visible commits that are below the rendering area
                let isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsVisible.length > 0) {
                        const pixel_y = self.commitsVisible[self.commitsVisible.length - 1]['pixel_y'];
                        if (pixel_y > renderingAreaBottom) {
                            self.commitsBelow.unshift(self.commitsVisible.pop());
                        } else {
                            isInRenderingArea = true;
                        }
                    } else {
                        // We're not technically in the rendering area but we need to exit the loop
                        // because there are no more visible commits.
                        isInRenderingArea = true;
                    }
                }

                // Add above commits to rendering area (if they're present there)
                isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsAbove.length > 0) {
                        const pixel_y = self.commitsAbove[self.commitsAbove.length - 1]['pixel_y'];
                        if (pixel_y > renderingAreaTop && pixel_y < renderingAreaBottom) {
                            self.commitsVisible.unshift(self.commitsAbove.pop());
                        } else if (pixel_y > renderingAreaBottom) {
                            self.commitsBelow.unshift(self.commitsAbove.pop());
                        } else {
                            isInRenderingArea = true;
                        }
                    } else {
                        isInRenderingArea = true;
                    }
                }
            } else if (renderingAreaTop > self.oldRenderingAreaTop) {
                // Scrolling down
                // Remove visible commits that are above the rendering area
                let isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsVisible.length > 0) {
                        const pixel_y = self.commitsVisible[0]['pixel_y'];
                        if (pixel_y < renderingAreaTop) {
                            self.commitsAbove.push(self.commitsVisible.shift());
                        } else {
                            isInRenderingArea = true;
                        }
                    } else {
                        // We're not technically in the rendering area but we need to exit the loop
                        // because there are no more visible commits.
                        isInRenderingArea = true;
                    }
                }

                // Add above commits to rendering area (if they're present there)
                isInRenderingArea = false;
                while (!isInRenderingArea) {
                    if (self.commitsBelow.length > 0) {
                        const pixel_y = self.commitsBelow[0]['pixel_y'];
                        if (pixel_y < renderingAreaBottom && pixel_y > renderingAreaTop) {
                            self.commitsVisible.push(self.commitsBelow.shift());
                        } else if (pixel_y < renderingAreaTop) {
                            self.commitsAbove.push(self.commitsBelow.shift());
                        } else {
                            isInRenderingArea = true;
                        }
                    } else {
                        isInRenderingArea = true;
                    }
                }
            }

            self.renderVisibleCommits();
            self.oldRenderingAreaTop = renderingAreaTop;
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

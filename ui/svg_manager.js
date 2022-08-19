import {writeText} from "@tauri-apps/api/clipboard";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    X_SPACING = 20;  // If changing, be sure to update on backend-end too
    X_OFFSET = 20;  // If changing, be sure to update on backend-end too
    BRANCH_TEXT_SPACING = 5;
    /**
     * Constructs the svg manager.
     */
    constructor() {
        this.$commitTableSVG = $('#commitTableSVG');
        this.repoInfo = [];
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

        self.$commitTableSVG.empty();
        self.$commitTableSVG.attr('height', self.repoInfo.length * 30);

        let maxWidth = 0;
        for (const commit of self.repoInfo) {
            for (const childLine of commit[0]) {
                const line = self.makeSVG(childLine['tag'], childLine['attrs']);
                self.$commitTableSVG.append(line);
            }
            const circle = self.makeSVG(commit[1]['tag'], commit[1]['attrs']);
            self.$commitTableSVG.append(circle);

            let currentX = -1;
            for (const branch_or_tags of commit[2]) {
                if (currentX === -1) {
                    currentX = (branch_or_tags[0]['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
                }
                branch_or_tags[0]['attrs']['x'] = currentX;
                branch_or_tags[1]['attrs']['x'] = currentX - 5;
                const txtElem = self.makeSVG(branch_or_tags[0]['tag'], branch_or_tags[0]['attrs']);
                const rectElem = self.makeSVG(branch_or_tags[1]['tag'], branch_or_tags[1]['attrs']);
                txtElem.textContent = branch_or_tags[0]['textContent'];
                self.$commitTableSVG.append(rectElem);
                self.$commitTableSVG.append(txtElem);
                const box_width = txtElem.getBBox().width + 10;
                const rectElemInHTML = document.getElementById(branch_or_tags[1]['attrs']['id'].toString());
                rectElemInHTML.setAttribute('width', box_width.toString());
                currentX += box_width + self.BRANCH_TEXT_SPACING;
            }

            if (currentX === -1) {
                currentX = (commit[3]['largestXValue'] + 1) * self.X_SPACING + self.X_OFFSET;
            }
            commit[3]['attrs']['x'] = currentX;
            const summaryTxt = self.makeSVG(commit[3]['tag'], commit[3]['attrs']);
            summaryTxt.textContent = commit[3]['textContent'];
            self.$commitTableSVG.append(summaryTxt);

            let width = currentX + summaryTxt.getBBox().width;
            commit[4]['attrs']['width'] = width;
            const backRect = self.makeSVG(commit[4]['tag'], commit[4]['attrs']);
            self.$commitTableSVG.append(backRect);

            maxWidth = Math.max(maxWidth, width);
        }

        self.$commitTableSVG.attr('width', maxWidth);
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

import {SVGRow} from "./svg_row";

/**
 * A class to manage the svg element.
 */
export class SVGManager {
    /**
     * Constructs the svg manager.
     */
    constructor() {
        this.$commitTableSVG = $('#commitTableSVG');
        this.entryResults = [];
    }

    /**
     * Refreshes the commit table with new entry results.
     */
    updateCommitTable(entryResults) {
        this.entryResults = entryResults;
        this.refreshCommitTable();
    }

    /**
     * Refreshes the commit table. Can be called on its own for a passive refresh.
     */
    refreshCommitTable() {
        const self = this;

        self.$commitTableSVG.empty();
        self.$commitTableSVG.attr('height', self.entryResults.length * 30);

        const svgRows = [];
        for (const entry of self.entryResults) {
            svgRows.push(new SVGRow(entry[1][2], entry[1][3], entry[1][4], entry[1][0][0], entry[1][0][1], entry));
        }

        let maxWidth = 0;
        const mainTable = {};
        for (const svgRow of svgRows) {
            svgRow.draw(self.$commitTableSVG, svgRow.getParentSVGRows(svgRows), svgRow.getChildSVGRows(svgRows), mainTable);
            maxWidth = Math.max(maxWidth, svgRow.width);
        }

        self.$commitTableSVG.attr('width', maxWidth);
    }
}

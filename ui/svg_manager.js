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

        const svgRows = [];
        for (const commit of self.repoInfo) {
            svgRows.push(new SVGRow(commit['oid'], commit['summary'], commit['branches_and_tags'], commit['parent_oids'], commit['child_oids'], commit['x'], commit['y']));
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

<template>
  <div class="resizable-column resizeable-column-branches full-height">
    <ul id="branchesTreeView" class="tree-view">
      <li><span class="parent-tree"><i class="fa-solid fa-caret-down rotated-caret"></i> Local Branches</span>
        <ul id="localBranches" class="nested sub-tree-view active-tree"></ul>
      </li>
      <li><span id="remoteBranchesHeader" class="parent-tree"><i class="fa-solid fa-caret-down"></i> Remote Branches</span>
        <ul id="remoteBranches" class="nested sub-tree-view"></ul>
      </li>
      <li><span class="parent-tree"><i class="fa-solid fa-caret-down"></i> Tags</span>
        <ul id="tags" class="nested sub-tree-view"></ul>
      </li>
      <li><span class="parent-tree"><i class="fa-solid fa-caret-down"></i> Stashes</span>
        <ul id="stashes" class="nested sub-tree-view"></ul>
      </li>
    </ul>
  </div>
</template>
    
<style scoped>
/* Remove default bullets */
.sub-tree-view, .tree-view {
  list-style-type: none;
}

/* Remove margins and padding from the parent ul */
.tree-view {
  margin: 0;
  padding: 0;
}

.sub-tree-view {
  padding-left: 20px;
}

.fa-caret-down {
  display: inline-block;
  transform: rotate(270deg);
}

.parent-tree, .inner-branch-item {
  white-space: nowrap;
  cursor: pointer;
}

/* Rotate the caret/arrow icon when clicked on (using JavaScript) */
.rotated-caret {
  transform: rotate(0deg);
}

/* Hide the nested list */
.nested {
  display: none;
}

/* Show the nested list when the user clicks on the caret/arrow (with JavaScript) */
.active-tree {
  display: block;
}
</style>
<template>
  <div class="flex-auto-in-row display-flex-column">
    <ul class="nav nav-tabs">
      <li class="nav-item">
        <button class="nav-link tab-btn-xsm active" id="commits-tab" data-bs-toggle="tab" data-bs-target="#commits" type="button" role="tab" aria-controls="commits" aria-selected="true">Commit Tree</button>
      </li>
      <li class="nav-item">
        <button class="nav-link tab-btn-xsm" id="changes-tab" data-bs-toggle="tab" data-bs-target="#changes" type="button" role="tab" aria-controls="changes" aria-selected="true">Changes</button>
      </li>
    </ul>
    
    <div class="tab-content flex-auto-in-column">
      <!-- Commit tree -->
      <div class="tab-pane fade show active full-height" id="commits" role="tabpanel" aria-labelledby="commits-tab">
        <div class="overflow-hidden full-height display-flex-column">
          <div class="resizable-row resizable-row-graph half-height full-width">
            <div class="full-height display-flex-column">
              <h4 id="projectName">Commits</h4>
              <div id="commitColumn" class="flex-auto-in-column overflow-auto">
                <svg id="commitTableSVG" width="0" height="0"></svg>
              </div>
            </div>
          </div>
          <div class="flex-auto-in-column display-flex-column">
            <ul class="nav nav-tabs display-flex-row flex-nowrap">
              <li class="nav-item">
                <button class="nav-link tab-btn-xsm white-space-nowrap active" id="commit-info-tab" data-bs-toggle="tab" data-bs-target="#commit-info" type="button" role="tab" aria-controls="commit-info" aria-selected="true">Info</button>
              </li>
              <li class="nav-item">
                <button class="nav-link tab-btn-xsm white-space-nowrap" id="commit-diff-tab" data-bs-toggle="tab" data-bs-target="#commit-diff" type="button" role="tab" aria-controls="commit-diff" aria-selected="true">File Changes</button>
              </li>
              <li class="nav-item little-padding-left flex-auto-in-row">
                <p id="commitWindowInfo" class="full-height no-margin-bottom display-flex-column white-space-nowrap"></p>
              </li>
            </ul>

              <div class="tab-content flex-auto-in-column">
                  <div class="tab-pane fade show active full-height overflow-auto" id="commit-info" role="tabpanel" aria-labelledby="commit-info-tab"></div>
                  <div class="tab-pane fade full-height" id="commit-diff" role="tabpanel" aria-labelledby="commit-diff-tab">
                      <div class="full-height display-flex-column">
                          <div class="flex-auto-in-column display-flex-row">
                              <div class="resizable-column resizable-column-file-paths full-height little-padding-left little-padding-top">
                                  <div id="commitChanges" class="full-height overflow-auto"></div>
                              </div>
                              <div id="commitFileDiffTableContainer" class="flex-auto-in-row full-height overflow-auto little-padding-top little-padding-left">
                                  <table id="commitFileDiffTable"></table>
                              </div>
                          </div>
                      </div>
                  </div>
              </div>
          </div>
        </div>
      </div>
      <!-- Changes View -->
      <div class="tab-pane fade full-height" id="changes" role="tabpanel" aria-labelledby="changes-tab">
          <div class="full-height display-flex-column">
              <div class="flex-auto-in-column display-flex-row">
                  <div class="resizable-column resizable-column-file-paths full-height little-padding-left little-padding-top">
                      <div class="overflow-hidden display-flex-column full-height">
                          <div class="resizable-row half-height full-width">
                              <div class="display-flex-column full-height">
                                  <div class="display-flex-row full-width">
                                      <h5>Unstaged Changes</h5>
                                      <div class="flex-auto-in-row">
                                          <button id="stageAllBtn" class="btn btn-success btn-sm right-padding" type="button">Stage All</button>
                                      </div>
                                  </div>
                                  <div id="unstagedChanges" class="flex-auto-in-column overflow-auto"></div>
                              </div>
                          </div>
                          <div class="flex-auto-in-column display-flex-column">
                              <h5>Staged Changes</h5>
                              <div id="stagedChanges" class="flex-auto-in-column overflow-auto"></div>
                          </div>
                      </div>
                  </div>
                  <div id="fileDiffTableContainer" class="flex-auto-in-row full-height overflow-auto little-padding-top little-padding-left">
                      <table id="fileDiffTable"></table>
                  </div>
              </div>
              <div id="commitControls" class="controls">
                  <div class="input-group">
                      <input id="summaryTxt" type="text" class="form-control bg-dark text-white" placeholder="Summary">
                      <span id="summaryTxtCounter" class="input-group-text bg-dark text-white"></span>
                  </div>
                  <textarea id="messageTxt" class="bg-dark text-white" rows="3" placeholder="Message"></textarea>
                  <span class="right">
                      <button id="commitBtn" class="btn btn-primary" type="button">
                          <i class="fa-regular fa-circle-check"></i> Commit
                      </button>
                  </span>
                  <span class="right-padding">
                      <button id="commitPushBtn" class="btn btn-primary" type="button">
                          <i class="fa-solid fa-arrow-up"></i> Commit & Push
                      </button>
                  </span>
              </div>
              <div id="mergeControls" class="controls">
                  <span class="right">
                      <button id="continueMergeBtn" class="btn btn-success" type="button">Continue Merge</button>
                  </span>
                  <span class="right-padding">
                      <button id="abortMergeBtn" class="btn btn-danger" type="button">Abort Merge</button>
                  </span>
              </div>
              <div id="rebaseControls" class="controls">
                  <span class="right">
                      <button id="continueRebaseBtn" class="btn btn-success" type="button">Continue Rebase</button>
                  </span>
                  <span class="right-padding">
                      <button id="abortRebaseBtn" class="btn btn-danger" type="button">Abort Rebase</button>
                  </span>
              </div>
              <div id="cherrypickControls" class="controls">
                  <span class="right">
                      <button id="continueCherrypickBtn" class="btn btn-success" type="button">Continue Cherrypick</button>
                  </span>
                  <span class="right-padding">
                      <button id="abortCherrypickBtn" class="btn btn-danger" type="button">Abort Cherrypick</button>
                  </span>
              </div>
              <div id="revertControls" class="controls">
                  <span class="right">
                      <button id="continueRevertBtn" class="btn btn-success" type="button">Continue Revert</button>
                  </span>
                  <span class="right-padding">
                      <button id="abortRevertBtn" class="btn btn-danger" type="button">Abort Revert</button>
                  </span>
              </div>
          </div>
      </div>
    </div>
  </div>
</template>
<template>
  <div class="display-flex-row">
    <div>
      <button id="fetchBtn" type="button" class="btn btn-secondary btn-sm"><i class="fa-solid fa-arrow-down"></i> Fetch</button>
      <button id="pullBtn" type="button" class="btn btn-primary btn-sm"><i class="fa-solid fa-arrow-down"></i> Pull</button>
      <button id="openPushModalBtn" type="button" class="btn btn-primary btn-sm"><i class="fa-solid fa-arrow-up"></i> Push</button>
      <button id="openStashModalBtn" type="button" class="btn btn-primary btn-sm"><i class="fa-solid fa-box-archive"></i> Stash</button>
      <button id="openBranchModalBtn" type="button" class="btn btn-primary btn-sm"><i class="fa-solid fa-code-branch"></i> Branch</button>
      <button id="openTagModalBtn" type="button" class="btn btn-primary btn-sm"><i class="fa-solid fa-tag"></i> Tag</button>
    </div>
    <div id="spinnerContainer">
      <div id="mainSpinner" class="spinner-border text-light" role="status">
          <span class="visually-hidden">Loading...</span>
      </div>
    </div>
  </div>
</template>
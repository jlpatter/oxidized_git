use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use anyhow::{bail, Result};
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use git2::{BranchType, Diff, ErrorCode, Oid, RepositoryState};
use serde::{Serialize, Deserialize, Serializer};
use crate::git_manager::GitManager;
use crate::svg_row::{RowProperty, SVGProperty, SVGRow};

#[derive(Clone)]
pub enum SVGCommitInfoValue {
    SomeString(String),
    SomeStringVec(Vec<String>),
    SomeStringTupleVec(Vec<(String, String)>),
    SomeInt(isize),
}

impl Serialize for SVGCommitInfoValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGCommitInfoValue::SomeString(st) => st.serialize(serializer),
            SVGCommitInfoValue::SomeStringVec(v) => v.serialize(serializer),
            SVGCommitInfoValue::SomeStringTupleVec(v) => v.serialize(serializer),
            SVGCommitInfoValue::SomeInt(i) => i.serialize(serializer),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct CommitsInfo {
    branch_draw_properties: Vec<(String, Vec<Vec<HashMap<String, SVGProperty>>>)>,
    svg_row_draw_properties: Vec<HashMap<String, RowProperty>>,
}

impl CommitsInfo {
    pub fn new(branch_draw_properties: Vec<(String, Vec<Vec<HashMap<String, SVGProperty>>>)>, svg_row_draw_properties: Vec<HashMap<String, RowProperty>>) -> Self {
        Self {
            branch_draw_properties,
            svg_row_draw_properties,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ParseableCommitInfo {
    sha: String,
    author_name: String,
    author_time: String,
    x: isize,
    y: isize,
    summary: String,
    parent_shas: Vec<String>,
    child_shas: Vec<String>,
}

impl ParseableCommitInfo {
    pub fn new(sha: String, author_name: String, author_time: String, x: isize, y: isize, summary: String, parent_shas: Vec<String>, child_shas: Vec<String>) -> Self {
        Self {
            sha,
            author_name,
            author_time,
            x,
            y,
            summary,
            parent_shas,
            child_shas,
        }
    }

    pub fn borrow_sha(&self) -> &String {
        &self.sha
    }

    pub fn borrow_author_name(&self) -> &String {
        &self.author_name
    }

    pub fn borrow_author_time(&self) -> &String {
        &self.author_time
    }

    pub fn borrow_x(&self) -> &isize {
        &self.x
    }

    pub fn borrow_y(&self) -> &isize {
        &self.y
    }

    pub fn borrow_summary(&self) -> &String {
        &self.summary
    }

    pub fn borrow_parent_shas(&self) -> &Vec<String> {
        &self.parent_shas
    }

    pub fn borrow_child_shas(&self) -> &Vec<String> {
        &self.child_shas
    }
}

#[derive(Clone)]
pub enum RepoInfoValue {
    SomeCommitInfo(CommitsInfo),
    SomeBranchInfo(BranchesInfo),
    SomeRemoteInfo(Vec<String>),
    SomeGeneralInfo(HashMap<String, String>),
    SomeFilesChangedInfo(FilesChangedInfo),
}

impl Serialize for RepoInfoValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            RepoInfoValue::SomeCommitInfo(c) => c.serialize(serializer),
            RepoInfoValue::SomeBranchInfo(b) => b.serialize(serializer),
            RepoInfoValue::SomeRemoteInfo(v) => v.serialize(serializer),
            RepoInfoValue::SomeGeneralInfo(hm) => hm.serialize(serializer),
            RepoInfoValue::SomeFilesChangedInfo(f) => f.serialize(serializer),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ParseableDiffDelta {
    status: u8,
    path: String,
}

impl ParseableDiffDelta {
    pub fn new(status: u8, path: String) -> Self {
        Self {
            status,
            path,
        }
    }

    pub fn get_status(&self) -> u8 {
        self.status
    }

    pub fn get_path(&self) -> &String {
        &self.path
    }
}

#[derive(Clone, Serialize)]
pub struct FilesChangedInfo {
    files_changed: usize,
    unstaged_files: Vec<ParseableDiffDelta>,
    staged_files: Vec<ParseableDiffDelta>,
}

impl FilesChangedInfo {
    pub fn new(files_changed: usize, unstaged_files: Vec<ParseableDiffDelta>, staged_files: Vec<ParseableDiffDelta>) -> Self {
        Self {
            files_changed,
            unstaged_files,
            staged_files,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct BranchInfo {
    target_sha: String,
    branch_shorthand: String,
    full_branch_name: String,
    is_head: bool,
    branch_type: String,
    ahead: usize,
    behind: usize,
    has_upstream: bool,
}

impl BranchInfo {
    pub fn new(target_sha: String, branch_shorthand: String, full_branch_name: String, is_head: bool, branch_type: String, ahead: usize, behind: usize, has_upstream: bool) -> Self {
        Self {
            target_sha,
            branch_shorthand,
            full_branch_name,
            is_head,
            branch_type,
            ahead,
            behind,
            has_upstream,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct StashInfo {
    index: usize,
    message: String,
}

impl StashInfo {
    pub fn new(index: usize, message: String) -> Self {
        Self {
            index,
            message,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct BranchInfoTreeNode {
    text: String,
    branch_info: Option<BranchInfo>,
    children: Vec<BranchInfoTreeNode>,
}

impl BranchInfoTreeNode {
    fn new(text: String, branch_info: Option<BranchInfo>) -> Self {
        Self {
            text,
            branch_info,
            children: vec![],
        }
    }

    pub fn insert_split_shorthand(&mut self, split_shorthand: VecDeque<String>, branch_info: Option<BranchInfo>) {
        // self should be the root node in this case.
        assert_eq!(self.text, String::from(""));
        let mut current_tree_node = self;

        for (i, string_ref) in split_shorthand.iter().enumerate() {
            let s = string_ref.clone();
            let child_index = current_tree_node.children.iter().position(|child| {
                child.text == s
            });
            match child_index {
                Some(j) => {
                    current_tree_node = &mut current_tree_node.children[j];
                },
                None => {
                    if i == split_shorthand.len() - 1 {
                        current_tree_node.children.push(BranchInfoTreeNode::new(s, branch_info.clone()));
                    } else {
                        current_tree_node.children.push(BranchInfoTreeNode::new(s, None));
                    }
                    let last_index = current_tree_node.children.len() - 1;
                    current_tree_node = &mut current_tree_node.children[last_index];
                },
            };
        }
    }
}

#[derive(Clone, Serialize)]
pub struct BranchesInfo {
    local_branch_info_tree: BranchInfoTreeNode,
    remote_branch_info_tree: BranchInfoTreeNode,
    tag_branch_info_tree: BranchInfoTreeNode,
    stash_info_list: Vec<StashInfo>,
}

impl BranchesInfo {
    pub fn new(local_branch_info_tree: BranchInfoTreeNode, remote_branch_info_tree: BranchInfoTreeNode, tag_branch_info_tree: BranchInfoTreeNode, stash_info_list: Vec<StashInfo>) -> Self {
        Self {
            local_branch_info_tree,
            remote_branch_info_tree,
            tag_branch_info_tree,
            stash_info_list,
        }
    }
}

fn get_oid_refs(git_manager: &GitManager) -> Result<HashMap<String, Vec<(String, String)>>> {
    let repo = git_manager.borrow_repo()?;

    // Get HashMap of Oids and their refs based on type (local, remote, or tag)
    let mut oid_refs: HashMap<String, Vec<(String, String)>> = HashMap::new();

    // Iterate over branches
    for branch_result in repo.branches(None)? {
        let (branch, _) = branch_result?;
        let mut branch_string = String::new();
        if branch.is_head() {
            branch_string.push_str("* ");
        }

        let reference = branch.get();
        let ref_shorthand = GitManager::get_utf8_string(reference.shorthand(), "Ref Name")?;
        branch_string.push_str(ref_shorthand);
        match reference.target() {
            Some(oid) => {
                let branch_type;
                if reference.is_remote() {
                    branch_type = "remote".to_string();
                } else {
                    branch_type = "local".to_string();
                }
                match oid_refs.get_mut(&*oid.to_string()) {
                    Some(oid_ref_vec) => {
                        oid_ref_vec.push((branch_string, branch_type));
                    },
                    None => {
                        oid_refs.insert(oid.to_string(), vec![(branch_string, branch_type)]);
                    },
                }
            },
            None => (),
        };
    }

    // If HEAD is detached, add it too
    if repo.head_detached()? {
        match repo.head()?.target() {
            Some(oid) => {
                match oid_refs.get_mut(&*oid.to_string()) {
                    Some(oid_ref_vec) => {
                        oid_ref_vec.push((String::from("* HEAD"), String::from("local")));
                    },
                    None => {
                        oid_refs.insert(oid.to_string(), vec![(String::from("* HEAD"), String::from("local"))]);
                    },
                }
            },
            None => (),
        };
    }

    // Iterate over tags
    for reference_result in repo.references()? {
        let reference = reference_result?;
        if reference.is_tag() {
            let ref_name = GitManager::get_utf8_string(reference.shorthand(), "Tag Name")?;

            let oid = reference.peel_to_commit()?.id();
            match oid_refs.get_mut(&*oid.to_string()) {
                Some(oid_ref_vec) => {
                    oid_ref_vec.push((ref_name.to_string(), "tag".to_string()));
                }
                None => {
                    oid_refs.insert(oid.to_string(), vec![(ref_name.to_string(), "tag".to_string())]);
                },
            };
        }
    }
    Ok(oid_refs)
}

fn get_general_info(git_manager: &GitManager) -> Result<HashMap<String, String>> {
    let repo = git_manager.borrow_repo()?;

    let mut general_info: HashMap<String, String> = HashMap::new();

    let project_name = match repo.workdir() {
        Some(p) => {
            match p.file_name() {
                Some(d) => d,
                None => bail!("Working directory path is empty?"),
            }
        },
        None => bail!("Repo doesn't have a working directory?"),
    };
    general_info.insert(String::from("project_name"), String::from(GitManager::get_utf8_string(project_name.to_str(), "Project Containing Directory")?));

    general_info.insert(String::from("head_sha"), String::new());
    match repo.head() {
        Ok(head_ref) => {
            if let Some(oid) = head_ref.target() {
                general_info.insert(String::from("head_sha"), oid.to_string());
            }

            match repo.find_branch(GitManager::get_utf8_string(head_ref.shorthand(), "Branch Name")?, BranchType::Local) {
                Ok(head_branch) => {
                    match head_branch.upstream() {
                        Ok(_) => {
                            general_info.insert(String::from("head_has_upstream"), true.to_string());
                        },
                        Err(e) => {
                            if e.code() == ErrorCode::NotFound {
                                general_info.insert(String::from("head_has_upstream"), false.to_string());
                            } else {
                                return Err(e.into());
                            }
                        },
                    }
                },
                Err(e) => {
                    if e.code() == ErrorCode::NotFound {
                        general_info.insert(String::from("head_has_upstream"), false.to_string());
                    } else {
                        return Err(e.into());
                    }
                },
            };
        },
        Err(e) => {
            if e.code() == ErrorCode::UnbornBranch {
                general_info.insert(String::from("head_has_upstream"), false.to_string());
            } else {
                return Err(e.into());
            }
        },
    };

    // Check if an operation is in progress (this means that conflicts occurred during the operation).
    let repo_state = repo.state();
    general_info.insert(String::from("is_cherrypicking"), (repo_state == RepositoryState::CherryPick).to_string());
    general_info.insert(String::from("is_reverting"), (repo_state == RepositoryState::Revert).to_string());
    general_info.insert(String::from("is_merging"), (repo_state == RepositoryState::Merge).to_string());
    general_info.insert(String::from("is_rebasing"), (repo_state == RepositoryState::Rebase || repo_state == RepositoryState::RebaseMerge || repo_state == RepositoryState::RebaseInteractive).to_string());

    Ok(general_info)
}

fn get_commit_info_list(git_manager: &GitManager, oid_list: Vec<Oid>) -> Result<Vec<ParseableCommitInfo>> {
    let mut commit_list: Vec<ParseableCommitInfo> = vec![];

    let repo = git_manager.borrow_repo()?;
    let mut children_oids_hm: HashMap<String, Vec<String>> = HashMap::new();
    for (i, oid) in oid_list.iter().enumerate() {
        let commit = repo.find_commit(*oid)?;

        // Get commit summary
        let commit_summary = GitManager::get_utf8_string(commit.summary(), "Commit Summary")?;

        // Get parent Oids
        let mut parent_shas: Vec<String> = vec![];
        for parent in commit.parents() {
            parent_shas.push(parent.id().to_string());
            match children_oids_hm.get_mut(&*parent.id().to_string()) {
                Some(children_oid_vec) => children_oid_vec.push(oid.to_string()),
                None => {
                    children_oids_hm.insert(parent.id().to_string(), vec![oid.to_string()]);
                },
            };
        }

        let author_signature = commit.author();
        let author_name = String::from(GitManager::get_utf8_string(author_signature.name(), "Author Name")?);

        let author_time = author_signature.when().seconds();
        let naive_datetime = match NaiveDateTime::from_timestamp_opt(author_time, 0) {
            Some(d) => d,
            None => bail!("Invalid Timestamp!"),
        };
        let utc_datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
        let local_datetime: DateTime<Local> = DateTime::from(utc_datetime);

        let naive_today = Local::now().date_naive();
        let diff = naive_today - local_datetime.date_naive();
        let formatted_datetime;
        if diff.num_days() == 0 {
            formatted_datetime = format!("Today {}", local_datetime.format("%r"));
        } else if diff.num_days() == 1 {
            formatted_datetime = format!("Yesterday {}", local_datetime.format("%r"));
        } else {
            formatted_datetime = format!("{}", local_datetime.format("%F %r"));
        }

        commit_list.push(ParseableCommitInfo::new(
            oid.to_string(),
            author_name,
            formatted_datetime,
            0,
            i as isize,
            String::from(commit_summary),
            parent_shas,
            vec![])
        );
    }

    // Gather the child commits after running through the commit graph once in order
    // to actually have populated entries.
    for commit_info in commit_list.iter_mut() {
        match children_oids_hm.get(&*commit_info.sha) {
            Some(v) => {
                commit_info.child_shas = v.clone();
            },
            None => (),
        };
    }

    Ok(commit_list)
}

fn get_commit_svg_draw_properties_list(git_manager: &mut GitManager, force_refresh: bool) -> Result<CommitsInfo> {
    let mut commit_info_list = vec![];
    if let Some(oid_vec) = git_manager.git_revwalk(force_refresh)? {
        commit_info_list = get_commit_info_list(git_manager, oid_vec)?;
    }

    let mut svg_row_draw_properties: Vec<HashMap<String, RowProperty>> = vec![];
    if commit_info_list.len() > 0 {
        let mut svg_rows: Vec<Rc<RefCell<SVGRow>>> = vec![];
        let mut svg_row_hm: HashMap<String, Rc<RefCell<SVGRow>>> = HashMap::new();
        for commit_info in commit_info_list {
            let svg_row_rc: Rc<RefCell<SVGRow>> = Rc::new(RefCell::new(SVGRow::from_commit_info(&commit_info)));
            svg_row_hm.insert(commit_info.sha.clone(), svg_row_rc.clone());
            svg_rows.push(svg_row_rc);
        }

        for svg_row_rc in &svg_rows {
            svg_row_rc.borrow_mut().set_parent_and_child_svg_row_values(&svg_row_hm);
        }

        let main_table = SVGRow::get_occupied_table(&svg_rows)?;
        for svg_row_rc in svg_rows {
            svg_row_draw_properties.push(svg_row_rc.borrow_mut().get_draw_properties(
                &main_table,
            ));
        }
    }

    let oid_refs_hm = get_oid_refs(git_manager)?;
    let mut branch_draw_properties: Vec<(String, Vec<Vec<HashMap<String, SVGProperty>>>)> = vec![];
    for (k, v) in oid_refs_hm {
        branch_draw_properties.push((k, SVGRow::get_branch_draw_properties(v)));
    }

    Ok(CommitsInfo::new(branch_draw_properties, svg_row_draw_properties))
}

fn get_branch_info_list(git_manager: &mut GitManager) -> Result<BranchesInfo> {
    let repo = git_manager.borrow_repo_mut()?;

    // Get all remote heads to be excluded from branches info
    let remotes = repo.remotes()?;
    let mut remote_heads: Vec<String> = vec![];
    for remote in remotes.iter() {
        let mut remote_head_name = String::from(GitManager::get_utf8_string(remote, "Remote Name")?);
        remote_head_name.push_str("/HEAD");
        remote_heads.push(remote_head_name);
    }

    let mut local_branch_info_tree = BranchInfoTreeNode::new(String::from(""), None);
    let mut remote_branch_info_tree = BranchInfoTreeNode::new(String::from(""), None);
    let mut tag_branch_info_tree = BranchInfoTreeNode::new(String::from(""), None);
    for reference_result in repo.references()? {
        let reference = reference_result?;

        let target_sha = match reference.peel_to_commit() {
            Ok(c) => c.id().to_string(),
            Err(_) => String::new(),
        };

        // Get branch name
        let branch_shorthand = String::from(GitManager::get_utf8_string(reference.shorthand(), "Branch Name")?);

        // If this is the remote head, don't add it to the branches info
        if remote_heads.contains(&branch_shorthand) {
            continue;
        }

        // Get full branch name
        let full_branch_name = String::from(GitManager::get_utf8_string(reference.name(), "Branch Name")?);

        // Get if branch is head
        let mut is_head = false;
        if reference.is_branch() {
            let local_branch = repo.find_branch(branch_shorthand.as_str(), BranchType::Local)?;
            if local_branch.is_head() {
                is_head = true;
            }
        }

        // Get branch type
        let mut branch_type = String::from("");
        if reference.is_branch() {
            branch_type = String::from("local");
        } else if reference.is_remote() {
            branch_type = String::from("remote");
        } else if reference.is_tag() {
            branch_type = String::from("tag");
        }

        // Get ahead/behind counts
        let mut ahead = 0;
        let mut behind = 0;
        let mut has_upstream = false;
        if reference.is_branch() {
            let local_branch = repo.find_branch(branch_shorthand.as_str(), BranchType::Local)?;
            match local_branch.upstream() {
                Ok(remote_branch) => {
                    has_upstream = true;
                    match local_branch.get().target() {
                        Some(local_oid) => {
                            match remote_branch.get().target() {
                                Some(remote_oid) => {
                                    let (a, b) = repo.graph_ahead_behind(local_oid, remote_oid)?;
                                    ahead = a;
                                    behind = b;
                                },
                                None => (),
                            };
                        },
                        None => (),
                    };
                },
                Err(e) => {
                    if e.code() != ErrorCode::NotFound {
                        return Err(e.into());
                    }
                },
            };
        }

        let mut split_shorthand = VecDeque::new();
        for s in branch_shorthand.split("/") {
            split_shorthand.push_back(String::from(s));
        }
        let branch_info = BranchInfo::new(target_sha, branch_shorthand, full_branch_name, is_head, branch_type.clone(), ahead, behind, has_upstream);
        if branch_type == String::from("local") {
            local_branch_info_tree.insert_split_shorthand(split_shorthand, Some(branch_info));
        } else if branch_type == String::from("remote") {
            remote_branch_info_tree.insert_split_shorthand(split_shorthand, Some(branch_info));
        } else if branch_type == String::from("tag") {
            tag_branch_info_tree.insert_split_shorthand(split_shorthand, Some(branch_info));
        }
    }

    // Add remote names in case a remote is present but has no branches.
    for remote in remotes.iter() {
        let remote_name = String::from(GitManager::get_utf8_string(remote, "Remote Name")?);
        let mut split_shorthand = VecDeque::new();
        split_shorthand.push_back(remote_name);
        remote_branch_info_tree.insert_split_shorthand(split_shorthand, None);
    }

    let mut stash_info_list = vec![];
    repo.stash_foreach(|stash_index, stash_message, _stash_oid| {
        let stash_info = StashInfo::new(stash_index, format!("{}: {}", stash_index, stash_message));
        stash_info_list.push(stash_info);
        true
    })?;

    Ok(BranchesInfo::new(local_branch_info_tree, remote_branch_info_tree, tag_branch_info_tree, stash_info_list))
}

fn get_remote_info_list(git_manager: &GitManager) -> Result<Vec<String>> {
    let repo = git_manager.borrow_repo()?;

    let mut remote_info_list = vec![];
    let remote_string_array = repo.remotes()?;

    for remote_name_opt in remote_string_array.iter() {
        let remote_name = GitManager::get_utf8_string(remote_name_opt, "Remote Name")?;
        remote_info_list.push(String::from(remote_name));
    }
    Ok(remote_info_list)
}

pub fn get_parseable_diff_delta(diff: Diff) -> Result<Vec<ParseableDiffDelta>> {
    let mut files: Vec<ParseableDiffDelta> = vec![];
    for delta in diff.deltas() {
        let status = delta.status() as u8;
        let path = match delta.new_file().path() {
            Some(p) => {
                match p.to_str() {
                    Some(s) => s,
                    None => bail!("File Path uses invalid unicode. Not sure how your file system isn't corrupted..."),
                }
            },
            None => bail!("Possible invalid file path? I'm not actually sure why this error would occur. It looks like git didn't store a file path with a file or something."),
        };
        files.push(ParseableDiffDelta::new(status, String::from(path)));
    }
    Ok(files)
}

pub fn get_files_changed_info_list(git_manager: &GitManager) -> Result<Option<FilesChangedInfo>> {
    if !git_manager.has_open_repo() {
        return Ok(None);
    }
    let unstaged_diff = git_manager.get_unstaged_changes()?;
    let staged_diff = git_manager.get_staged_changes()?;
    let files_changed = unstaged_diff.stats()?.files_changed() + staged_diff.stats()?.files_changed();
    Ok(Some(FilesChangedInfo::new(files_changed, get_parseable_diff_delta(unstaged_diff)?, get_parseable_diff_delta(staged_diff)?)))
}

pub fn get_parseable_repo_info(git_manager: &mut GitManager, force_refresh: bool) -> Result<Option<HashMap<String, RepoInfoValue>>> {
    if !git_manager.has_open_repo() {
        return Ok(None);
    }
    let mut repo_info: HashMap<String, RepoInfoValue> = HashMap::new();
    repo_info.insert(String::from("general_info"), RepoInfoValue::SomeGeneralInfo(get_general_info(git_manager)?));
    repo_info.insert(String::from("commit_info_list"), RepoInfoValue::SomeCommitInfo(get_commit_svg_draw_properties_list(git_manager, force_refresh)?));
    repo_info.insert(String::from("branch_info_list"), RepoInfoValue::SomeBranchInfo(get_branch_info_list(git_manager)?));
    repo_info.insert(String::from("remote_info_list"), RepoInfoValue::SomeRemoteInfo(get_remote_info_list(git_manager)?));
    if let Some(fcil) = get_files_changed_info_list(git_manager)? {
        repo_info.insert(String::from("files_changed_info_list"), RepoInfoValue::SomeFilesChangedInfo(fcil));
    } else {
        bail!("Changes couldn't find repo but repo_info could for some reason?");
    }
    Ok(Some(repo_info))
}

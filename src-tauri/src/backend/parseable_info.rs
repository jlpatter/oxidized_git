use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use anyhow::{bail, Result};
use git2::{BranchType, Diff, ErrorCode, Oid};
use serde::{Serialize, Deserialize, Serializer};
use super::git_manager::GitManager;
use super::svg_row::RowProperty;
use super::svg_row::SVGRow;

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

#[derive(Clone)]
pub enum RepoInfoValue {
    SomeCommitInfo(Vec<HashMap<String, RowProperty>>),
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
    branch_shorthand: String,
    full_branch_name: String,
    is_head: bool,
    branch_type: String,
    ahead: usize,
    behind: usize,
}

impl BranchInfo {
    pub fn new(branch_shorthand: String, full_branch_name: String, is_head: bool, branch_type: String, ahead: usize, behind: usize) -> Self {
        Self {
            branch_shorthand,
            full_branch_name,
            is_head,
            branch_type,
            ahead,
            behind,
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

    pub fn insert_split_shorthand(&mut self, split_shorthand: VecDeque<String>, branch_info: BranchInfo) {
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
                        current_tree_node.children.push(BranchInfoTreeNode::new(s, Some(branch_info.clone())));
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
}

impl BranchesInfo {
    pub fn new(local_branch_info_tree: BranchInfoTreeNode, remote_branch_info_tree: BranchInfoTreeNode, tag_branch_info_tree: BranchInfoTreeNode) -> Self {
        Self {
            local_branch_info_tree,
            remote_branch_info_tree,
            tag_branch_info_tree,
        }
    }
}

fn get_oid_refs(git_manager: &GitManager) -> Result<HashMap<String, Vec<(String, String)>>> {
    let repo = git_manager.get_repo()?;

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
                    }
                    None => {
                        oid_refs.insert(oid.to_string(), vec![(branch_string, branch_type)]);
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
    let repo = git_manager.get_repo()?;

    let mut general_info: HashMap<String, String> = HashMap::new();
    let head_ref = repo.head()?;
    let head_branch = repo.find_branch(GitManager::get_utf8_string(head_ref.shorthand(), "Branch Name")?, BranchType::Local)?;

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

    Ok(general_info)
}

fn get_commit_info_list(git_manager: &GitManager, oid_list: Vec<Oid>) -> Result<Vec<HashMap<String, SVGCommitInfoValue>>> {
    let repo = git_manager.get_repo()?;

    let mut commit_list: Vec<HashMap<String, SVGCommitInfoValue>> = vec![];
    let oid_refs_hm = get_oid_refs(git_manager)?;

    let mut children_oids: HashMap<String, Vec<String>> = HashMap::new();
    for (i, oid) in oid_list.iter().enumerate() {
        let mut commit_info: HashMap<String, SVGCommitInfoValue> = HashMap::new();
        commit_info.insert("oid".into(), SVGCommitInfoValue::SomeString(oid.to_string()));
        commit_info.insert("x".into(), SVGCommitInfoValue::SomeInt(0));
        commit_info.insert("y".into(), SVGCommitInfoValue::SomeInt(i as isize));

        let commit = repo.find_commit(*oid)?;

        // Get commit summary
        let commit_summary = GitManager::get_utf8_string(commit.summary(), "Commit Summary")?;
        commit_info.insert("summary".into(), SVGCommitInfoValue::SomeString(commit_summary.into()));

        // Get branches pointing to this commit
        match oid_refs_hm.get(&*oid.to_string()) {
            Some(ref_vec) => {
                commit_info.insert("branches_and_tags".into(), SVGCommitInfoValue::SomeStringTupleVec(ref_vec.clone()));
            }
            None => {
                commit_info.insert("branches_and_tags".into(), SVGCommitInfoValue::SomeStringTupleVec(vec![]));
            },
        };

        // Get parent Oids
        let mut parent_oids: Vec<String> = vec![];
        for parent in commit.parents() {
            parent_oids.push(parent.id().to_string());
            match children_oids.get_mut(&*parent.id().to_string()) {
                Some(children_oid_vec) => children_oid_vec.push(oid.to_string()),
                None => {
                    children_oids.insert(parent.id().to_string(), vec![oid.to_string()]);
                },
            };
        }

        commit_info.insert("parent_oids".into(), SVGCommitInfoValue::SomeStringVec(parent_oids));
        commit_info.insert("child_oids".into(), SVGCommitInfoValue::SomeStringVec(vec![]));
        commit_list.push(commit_info);
    }

    // Gather the child commits after running through the commit graph once in order
    // to actually have populated entries.
    for commit_hm in commit_list.iter_mut() {
        let oid_string = match commit_hm.get("oid") {
            Some(oid) => {
                match oid {
                    SVGCommitInfoValue::SomeString(oid_string) => oid_string,
                    SVGCommitInfoValue::SomeStringVec(_some_vector) => bail!("Oid was stored as a vector instead of a string."),
                    SVGCommitInfoValue::SomeStringTupleVec(_some_hm) => bail!("Oid was stored as a hashmap instead of a string."),
                    SVGCommitInfoValue::SomeInt(_some_int) => bail!("Oid was stored as an int instead of a string."),
                }
            },
            None => bail!("Commit found with no oid, shouldn't be possible..."),
        };
        match children_oids.get(oid_string) {
            Some(v) => {
                commit_hm.insert("child_oids".into(), SVGCommitInfoValue::SomeStringVec(v.clone()));
            },
            None => (),
        };
    }

    Ok(commit_list)
}

fn get_branch_info_list(git_manager: &GitManager) -> Result<BranchesInfo> {
    let repo = git_manager.get_repo()?;

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

        // Get branch name
        let branch_shorthand = String::from(GitManager::get_utf8_string(reference.shorthand(), "Branch Name")?);

        // If this is the remote head, don't add it to the branches info
        let is_remote_head = remote_heads.iter().any(|head_name| {
            branch_shorthand == *head_name
        });
        if is_remote_head {
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
        if reference.is_branch() {
            let local_branch = repo.find_branch(branch_shorthand.as_str(), BranchType::Local)?;
            match local_branch.upstream() {
                Ok(remote_branch) => {
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
        let branch_info = BranchInfo::new(branch_shorthand, full_branch_name, is_head, branch_type.clone(), ahead, behind);
        if branch_type == String::from("local") {
            local_branch_info_tree.insert_split_shorthand(split_shorthand, branch_info);
        } else if branch_type == String::from("remote") {
            remote_branch_info_tree.insert_split_shorthand(split_shorthand, branch_info);
        } else if branch_type == String::from("tag") {
            tag_branch_info_tree.insert_split_shorthand(split_shorthand, branch_info);
        }
    }

    Ok(BranchesInfo::new(local_branch_info_tree, remote_branch_info_tree, tag_branch_info_tree))
}

fn get_remote_info_list(git_manager: &GitManager) -> Result<Vec<String>> {
    let repo = git_manager.get_repo()?;

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

pub fn get_parseable_repo_info(git_manager: &GitManager) -> Result<Option<HashMap<String, RepoInfoValue>>> {
    if !git_manager.has_open_repo() {
        return Ok(None);
    }
    let mut repo_info: HashMap<String, RepoInfoValue> = HashMap::new();
    let commit_info_list = get_commit_info_list(git_manager, git_manager.git_revwalk()?)?;
    let mut svg_rows: Vec<Rc<RefCell<SVGRow>>> = vec![];
    let mut svg_row_hm: HashMap<String, Rc<RefCell<SVGRow>>> = HashMap::new();
    for commit_info in commit_info_list {
        let oid = match commit_info.get("oid") {
            Some(civ_oid) => {
                if let SVGCommitInfoValue::SomeString(s) = civ_oid {
                    s
                } else {
                    bail!("Oid was not passed as a string.");
                }
            },
            None => bail!("Oid not found in commit_info hash map."),
        };
        let summary = match commit_info.get("summary") {
            Some(civ_summary) => {
                if let SVGCommitInfoValue::SomeString(s) = civ_summary {
                    s
                } else {
                    bail!("Summary was not passed as a string.");
                }
            }
            None => bail!("Summary not found in commit_info hash map."),
        };
        let branches_and_tags = match commit_info.get("branches_and_tags") {
            Some(civ_branches_and_tags) => {
                if let SVGCommitInfoValue::SomeStringTupleVec(v) = civ_branches_and_tags {
                    v
                } else {
                    bail!("branches_and_tags was not passed as a vector.");
                }
            }
            None => bail!("branches_and_tags not found in commit_info hash map."),
        };
        let parent_oids = match commit_info.get("parent_oids") {
            Some(civ_parent_oids) => {
                if let SVGCommitInfoValue::SomeStringVec(v) = civ_parent_oids {
                    v
                } else {
                    bail!("Parent Oids was not passed as a vector.");
                }
            }
            None => bail!("Parent Oids not found in commit_info hash map."),
        };
        let child_oids = match commit_info.get("child_oids") {
            Some(civ_child_oids) => {
                if let SVGCommitInfoValue::SomeStringVec(v) = civ_child_oids {
                    v
                } else {
                    bail!("Child Oids was not passed as a vector.");
                }
            }
            None => bail!("Child Oids not found in commit_info hash map."),
        };
        let x = match commit_info.get("x") {
            Some(civ_x) => {
                if let SVGCommitInfoValue::SomeInt(i) = civ_x {
                    i
                } else {
                    bail!("X was not passed as an isize.");
                }
            }
            None => bail!("X not found in commit_info hash map."),
        };
        let y = match commit_info.get("y") {
            Some(civ_y) => {
                if let SVGCommitInfoValue::SomeInt(i) = civ_y {
                    i
                } else {
                    bail!("Y was not passed as an isize.");
                }
            }
            None => bail!("Y not found in commit_info hash map."),
        };
        let svg_row_rc: Rc<RefCell<SVGRow>> = Rc::new(RefCell::new(SVGRow::new(
            oid.clone(),
            summary.clone(),
            branches_and_tags.clone(),
            parent_oids.clone(),
            child_oids.clone(),
            x.clone(),
            y.clone(),
        )));
        svg_row_hm.insert(oid.clone(), svg_row_rc.clone());
        svg_rows.push(svg_row_rc);
    }

    for svg_row_rc in &svg_rows {
        svg_row_rc.borrow_mut().set_parent_and_child_svg_row_values(&svg_row_hm);
    }

    let main_table = SVGRow::get_occupied_table(&mut svg_rows)?;
    let mut svg_row_draw_properties: Vec<HashMap<String, RowProperty>> = vec![];
    for svg_row_rc in svg_rows {
        svg_row_draw_properties.push(svg_row_rc.borrow_mut().get_draw_properties(
            &main_table,
        ));
    }

    repo_info.insert(String::from("general_info"), RepoInfoValue::SomeGeneralInfo(get_general_info(git_manager)?));
    repo_info.insert(String::from("commit_info_list"), RepoInfoValue::SomeCommitInfo(svg_row_draw_properties));
    repo_info.insert(String::from("branch_info_list"), RepoInfoValue::SomeBranchInfo(get_branch_info_list(git_manager)?));
    repo_info.insert(String::from("remote_info_list"), RepoInfoValue::SomeRemoteInfo(get_remote_info_list(git_manager)?));
    if let Some(fcil) = get_files_changed_info_list(git_manager)? {
        repo_info.insert(String::from("files_changed_info_list"), RepoInfoValue::SomeFilesChangedInfo(fcil));
    } else {
        bail!("Changes couldn't find repo but repo_info could for some reason?");
    }
    Ok(Some(repo_info))
}

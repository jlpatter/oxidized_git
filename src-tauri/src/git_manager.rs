use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::str;
use anyhow::{bail, Result};
use directories::BaseDirs;
use git2::{AutotagOption, BranchType, Commit, Cred, Delta, Diff, DiffFindOptions, DiffLine, DiffOptions, FetchOptions, FetchPrune, IndexAddOption, Oid, Patch, PushOptions, Rebase, Reference, RemoteCallbacks, Repository, ResetType, Signature, Sort};
use git2::build::CheckoutBuilder;
use rfd::FileDialog;
use serde::Serialize;
use crate::parseable_info::{get_parseable_diff_delta, ParseableDiffDelta};
use crate::config_manager;

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
    }
    if s.ends_with('\r') {
        s.pop();
    }
}

#[derive(Clone, Serialize)]
pub struct FileLineInfo {
    old_lineno: Option<u32>,
    new_lineno: Option<u32>,
    file_type: String,
    content: String,
    origin: char,
}

impl FileLineInfo {
    pub fn from_diff_line(diff_line: DiffLine, file_type: &String) -> Result<Self> {
        let mut content_string = String::from(str::from_utf8(diff_line.content())?);
        trim_newline(&mut content_string);
        content_string = html_escape::encode_text(&content_string).parse()?;
        let new_info = Self {
            old_lineno: diff_line.old_lineno(),
            new_lineno: diff_line.new_lineno(),
            file_type: file_type.clone(),
            content: content_string,
            origin: diff_line.origin(),
        };
        Ok(new_info)
    }
}

#[derive(Clone, Serialize)]
pub struct FileInfo {
    change_type: String,
    file_lines: Vec<FileLineInfo>,
}

impl FileInfo {
    pub fn new(change_type: String, file_lines: Vec<FileLineInfo>) -> Self {
        Self {
            change_type,
            file_lines,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct CommitInfo {
    sha: String,
    summary: String,
    message: String,
    author_name: String,
    author_time: i64,
    committer_name: String,
    committer_time: i64,
    changed_files: Vec<ParseableDiffDelta>,
}

impl CommitInfo {
    pub fn from_commit(commit: Commit, repo: &Repository) -> Result<Self> {
        let author_signature = commit.author();
        let author_name = String::from(GitManager::get_utf8_string(author_signature.name(), "Author Name")?);
        let author_time = author_signature.when().seconds();

        let committer_signature = commit.committer();
        let committer_name = String::from(GitManager::get_utf8_string(committer_signature.name(), "Committer Name")?);
        let committer_time = committer_signature.when().seconds();

        let diff = get_commit_changes(&commit, repo)?;
        let parseable_diff_delta = get_parseable_diff_delta(diff)?;

        let new_commit_info = Self {
            sha: commit.id().to_string(),
            summary: html_escape::encode_text(&String::from(GitManager::get_utf8_string(commit.summary(), "Commit Summary")?)).parse()?,
            message: html_escape::encode_text(&String::from(GitManager::get_utf8_string(commit.message(), "Commit Message")?)).parse()?,
            author_name: html_escape::encode_text(&author_name).parse()?,
            author_time,
            committer_name: html_escape::encode_text(&committer_name).parse()?,
            committer_time,
            changed_files: parseable_diff_delta,
        };

        Ok(new_commit_info)
    }
}

fn get_commit_changes<'a, 'b>(commit: &'a Commit, repo: &'b Repository) -> Result<Diff<'b>> {
    let commit_tree = commit.tree()?;

    for parent_commit in commit.parents() {
        let mut diff = repo.diff_tree_to_tree(Some(&parent_commit.tree()?), Some(&commit_tree), None)?;
        GitManager::set_diff_find_similar(&mut diff)?;
        // For merge commits, the diff between a merge commit and the parent from the branch that was merged will be empty,
        // so find the diff that's populated.
        if diff.stats()?.files_changed() > 0 {
            return Ok(diff);
        }
    }

    // If there are no parents, get the diff between this commit and nothing.
    let mut diff = repo.diff_tree_to_tree(None, Some(&commit_tree), None)?;
    GitManager::set_diff_find_similar(&mut diff)?;

    Ok(diff)
}

#[derive(PartialEq)]
pub enum GraphOps {
    AddedOnly,
    DeletedOnly,
    Both,
    RefChange,
    ConfigChange,
    DifferentRepo,
}

#[derive(Clone, Serialize)]
pub struct SHAChanges {
    clear_entire_old_graph: bool,
    created: Vec<String>,
    deleted: Vec<String>,
}

impl SHAChanges {
    pub fn new() -> Self {
        Self {
            clear_entire_old_graph: false,
            created: vec![],
            deleted: vec![],
        }
    }

    pub fn push_created(&mut self, sha: String) {
        self.created.push(sha);
    }

    pub fn push_deleted(&mut self, sha: String) {
        self.deleted.push(sha);
    }

    pub fn borrow_created(&self) -> &Vec<String> {
        &self.created
    }

    pub fn borrow_deleted(&self) -> &Vec<String> {
        &self.deleted
    }

    pub fn borrow_clear_entire_old_graph(&self) -> &bool {
        &self.clear_entire_old_graph
    }
}

pub struct GitManager {
    repo: Option<Repository>,
    sha_from_commit_from_op: Option<String>,
    old_graph_starting_shas: Vec<String>,
    old_revwalk_shas: VecDeque<String>,
}

impl GitManager {
    pub fn new() -> Self {
        Self {
            repo: None,
            sha_from_commit_from_op: None,
            old_graph_starting_shas: vec![],
            old_revwalk_shas: VecDeque::new(),
        }
    }

    pub fn get_utf8_string<'a, 'b>(value: Option<&'a str>, str_name_type: &'b str) -> Result<&'a str> {
        match value {
            Some(n) => Ok(n),
            None => bail!(format!("{} uses invalid utf-8!", str_name_type)),
        }
    }

    pub fn borrow_repo(&self) -> Result<&Repository> {
        let repo_temp_opt = &self.repo;
        match repo_temp_opt {
            Some(repo) => Ok(repo),
            None => bail!("No repo loaded to perform operation on."),
        }
    }

    pub fn has_open_repo(&self) -> bool {
        match self.borrow_repo() {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get_directory(&self) -> Option<PathBuf> {
        let bd_opt = BaseDirs::new();
        match bd_opt {
            Some(bd) => FileDialog::new().set_directory(bd.home_dir()).pick_folder(),
            None => FileDialog::new().set_directory(PathBuf::from("/")).pick_folder(),
        }
    }

    pub fn init_repo(&mut self) -> Result<bool> {
        match self.get_directory() {
            Some(path_buffer) => {
                self.repo = Some(Repository::init(path_buffer.as_path())?);
                Ok(true)
            },
            None => Ok(false),
        }
    }

    pub fn open_repo(&mut self) -> Result<bool> {
        match self.get_directory() {
            Some(path_buffer) => {
                self.repo = Some(Repository::open(path_buffer.as_path())?);
                Ok(true)
            },
            None => Ok(false),
        }
    }

    fn cleanup_state(&mut self) -> Result<()> {
        let repo = self.borrow_repo()?;
        repo.cleanup_state()?;
        self.sha_from_commit_from_op = None;
        Ok(())
    }

    fn get_message_without_summary(full_message: &str) -> (String, bool) {
        let mut separate_pieces: VecDeque<&str> = full_message.split("\r\n\r\n").collect();
        let mut use_crlf = true;

        if separate_pieces.len() == 1 {
            separate_pieces = full_message.split("\n\n").collect();
            use_crlf = false;
        }

        // Remove the summary
        separate_pieces.pop_front();

        if use_crlf {
            (separate_pieces.make_contiguous().join("\r\n\r\n"), use_crlf)
        } else {
            (separate_pieces.make_contiguous().join("\n\n"), use_crlf)
        }
    }

    fn old_shas_eq_sorted_new_oids(&self, new_oids: &Vec<Oid>) -> bool {
        // NOTE: This function assumes the new_oids are sorted by date!
        if self.old_graph_starting_shas.len() == new_oids.len() {
            for i in 0..self.old_graph_starting_shas.len() {
                if self.old_graph_starting_shas[i] != new_oids[i].to_string() {
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub fn git_revwalk(&mut self, commit_ops: GraphOps) -> Result<SHAChanges> {
        let mut oid_vec: Vec<Oid> = vec![];
        // This closure allows self to be borrowed mutably later for setting the new graph starting shas.
        {
            let repo = self.borrow_repo()?;
            for branch_result in repo.branches(None)? {
                let (branch, _) = branch_result?;
                match branch.get().target() {
                    Some(oid) => {
                        if !oid_vec.contains(&oid) {
                            oid_vec.push(oid);
                        }
                    },
                    None => (),
                };
            };

            if repo.head_detached()? {
                match repo.head()?.target() {
                    Some(oid) => {
                        if !oid_vec.contains(&oid) {
                            oid_vec.push(oid);
                        }
                    },
                    None => (),
                };
            }

            // Sort Oids by date first
            oid_vec.sort_by(|a, b| {
                repo.find_commit(*b).unwrap().time().seconds().partial_cmp(&repo.find_commit(*a).unwrap().time().seconds()).unwrap()
            });
        }

        let mut sha_changes = SHAChanges::new();
        if commit_ops == GraphOps::DifferentRepo || commit_ops == GraphOps::ConfigChange {
            self.old_graph_starting_shas = vec![];
            self.old_revwalk_shas = VecDeque::new();
            sha_changes.clear_entire_old_graph = true;
        }

        if self.old_shas_eq_sorted_new_oids(&oid_vec) {
            return Ok(SHAChanges::new());
        }

        // If you've reached here, the old and new starting oids are different. Update the old and perform the revwalk.
        self.old_graph_starting_shas = oid_vec.iter().map(|new_oid| {
            new_oid.to_string()
        }).collect();

        let repo = self.borrow_repo()?;
        let mut revwalk = repo.revwalk()?;

        // Sort topologically but each topology is sorted by date.
        for oid in oid_vec {
            revwalk.push(oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;

        let preferences = config_manager::get_preferences()?;
        let limit_commits = preferences.get_limit_commits();
        let commit_count = preferences.get_commit_count();

        let mut is_adding = false;
        let mut top_commit_topography_count: usize = 0;
        for (i, commit_oid_result) in revwalk.enumerate() {
            if limit_commits && i >= commit_count {
                break;
            }
            let oid = commit_oid_result?;
            let sha = oid.to_string();

            if self.old_revwalk_shas.len() > 0 {
                // If it's the first commit in the graph, it may have been added or deleted.
                if i == 0 && sha != self.old_revwalk_shas[i] {
                    let first_non_deleted_commit = self.old_revwalk_shas.iter().position(|old_sha| {
                        *old_sha == sha
                    });
                    match first_non_deleted_commit {
                        Some(j) => {
                            for k in i..j {
                                sha_changes.push_deleted(self.old_revwalk_shas[k].clone());
                            }
                            break;
                        },
                        None => {
                            is_adding = true;
                            sha_changes.push_created(sha.clone());
                            top_commit_topography_count += 1;
                        },
                    }
                } else if i > 0 && !is_adding && sha != self.old_revwalk_shas[i - top_commit_topography_count] {
                    // If we're not adding and there's a difference, then there's commit(s) to remove from the graph.
                    let first_non_deleted_commit = self.old_revwalk_shas.iter().position(|old_sha| {
                        *old_sha == sha
                    });
                    match first_non_deleted_commit {
                        Some(j) => {
                            for k in (i - top_commit_topography_count)..j {
                                sha_changes.push_deleted(self.old_revwalk_shas[k].clone());
                            }
                            break;
                        },
                        None => {
                            println!("I didn't think this code path was possible but here we are...");
                            println!("NOTE: This codepath implies somebody deleted the initial commit at the bottom of the graph!!!");
                            for k in (i - top_commit_topography_count)..self.old_revwalk_shas.len() {
                                sha_changes.push_deleted(self.old_revwalk_shas[k].clone());
                            }
                            break;
                        },
                    }
                } else if i > 0 && is_adding {
                    // If we're currently adding, add commit to the graph.
                    if self.old_revwalk_shas[i - top_commit_topography_count] == sha {
                        // When the revwalk reaches the last added commit.
                        is_adding = false;
                        if commit_ops == GraphOps::AddedOnly {
                            break;
                        }
                    } else {
                        sha_changes.push_created(sha.clone());
                        top_commit_topography_count += 1;
                        if self.old_revwalk_shas.contains(&sha) {
                            sha_changes.push_deleted(sha.clone());
                        }
                    }
                }
            } else {
                // This runs if the graph was previously empty.
                sha_changes.push_created(sha);
            }
        }

        for deleted_change in sha_changes.borrow_deleted() {
            let index_opt = self.old_revwalk_shas.iter().position(|old_sha| {
                *old_sha == *deleted_change
            });
            match index_opt {
                Some(i) => {
                    self.old_revwalk_shas.remove(i);
                },
                None => bail!("Deleted change not found in old revwalk, this should technically be impossible..."),
            }
        }

        // NOTE: This always assumes created commits are at the top of the graph.
        // this is due to the way the graph is sorted.
        // Need to reverse order first before inserting.
        let mut created_shas = VecDeque::new();
        for created_change in sha_changes.borrow_created() {
            created_shas.push_front(created_change.clone());
        }
        for sha in created_shas {
            self.old_revwalk_shas.push_front(sha);
        }

        Ok(sha_changes)
    }

    pub fn get_ref_from_name(&self, ref_full_name: &str) -> Result<Reference> {
        Ok(self.borrow_repo()?.find_reference(ref_full_name)?)
    }

    pub fn get_commit_info(&self, sha: &str) -> Result<CommitInfo> {
        let repo = self.borrow_repo()?;

        let commit = repo.find_commit(Oid::from_str(sha)?)?;
        let commit_info = CommitInfo::from_commit(commit, repo)?;

        Ok(commit_info)
    }

    fn has_conflicts(&self) -> Result<bool> {
        let unstaged_diff = self.get_unstaged_changes()?;
        let staged_diff = self.get_staged_changes()?;

        for delta in unstaged_diff.deltas() {
            if delta.status() == Delta::Conflicted {
                return Ok(true);
            }
        }

        for delta in staged_diff.deltas() {
            if delta.status() == Delta::Conflicted {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn has_unstaged_changes(&self) -> Result<bool> {
        let diff = self.get_unstaged_changes()?;

        if diff.stats()?.files_changed() > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn has_staged_changes(&self) -> Result<bool> {
        let diff = self.get_staged_changes()?;

        if diff.stats()?.files_changed() > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn git_merge(&mut self, sha: &str) -> Result<()> {
        // This closure allows self to be borrowed mutably for cleanup.
        {
            let repo = self.borrow_repo()?;
            let annotated_commit = repo.find_annotated_commit(Oid::from_str(sha)?)?;

            repo.merge(&[&annotated_commit], None, None)?;
        }

        if !self.has_conflicts()? {
            if self.has_staged_changes()? {
                let repo = self.borrow_repo()?;

                let head_commit = match repo.head()?.target() {
                    Some(oid) => repo.find_commit(oid)?,
                    None => bail!("HEAD has no target, failed to commit after merging. It should fail earlier than this since there'd be no HEAD to merge into."),
                };
                let merge_parent_two = repo.find_commit(Oid::from_str(sha)?)?;
                let parent_commits = vec![&head_commit, &merge_parent_two];
                let committer = repo.signature()?;

                let mut message = String::from("Merge commit ");
                let mut short_sha = String::from(sha);
                short_sha.truncate(5);
                message.push_str(&*short_sha);
                message.push_str(" into commit ");
                let mut head_short_sha = head_commit.id().to_string();
                head_short_sha.truncate(5);
                message.push_str(&*head_short_sha);

                self.git_commit(message, &committer, &committer, parent_commits)?;
            } else {
                bail!("Merge commit had no changes, not really sure what would cause this...");
            }
            self.cleanup_state()?;
        } else {
            self.sha_from_commit_from_op = Some(String::from(sha.clone()));
        }

        Ok(())
    }

    fn iterate_through_rebase(&self, repo: &Repository, rebase: &mut Rebase) -> Result<()> {
        // Unfortunately, using 'rebase' like an iterator doesn't allow us to commit since
        // rebase has to be borrowed mutably to commit.
        let mut reached_end = false;
        while !reached_end {
            let step = rebase.next();
            match step {
                Some(r) => {
                    r?;

                    // If there are conflicts, need to let the user fix them before resuming
                    // inside "git_continue_rebase"
                    if self.has_conflicts()? || self.has_unstaged_changes()? {
                        return Ok(());
                    } else if self.has_staged_changes()? {
                        rebase.commit(None, &repo.signature()?, None)?;
                    }
                },
                None => reached_end = true,
            };
        }
        rebase.finish(None)?;

        Ok(())
    }

    pub fn git_rebase(&self, sha: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        let annotated_commit = repo.find_annotated_commit(Oid::from_str(sha)?)?;
        let mut rebase = repo.rebase(None, None, Some(&annotated_commit), None)?;

        self.iterate_through_rebase(repo, &mut rebase)?;

        Ok(())
    }

    pub fn git_cherrypick(&mut self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha wasn't included in payload from the front-end."),
        };
        let is_committing = match json_hm.get("isCommitting") {
            Some(s) => s == "true",
            None => bail!("isCommitting wasn't included in payload from the front-end."),
        };

        // This closure allows self to be borrowed mutably for cleanup.
        {
            let repo = self.borrow_repo()?;
            let commit = repo.find_commit(Oid::from_str(sha)?)?;

            repo.cherrypick(&commit, None)?;
        }

        if !self.has_conflicts()? {
            self.cleanup_state()?;
        } else {
            self.sha_from_commit_from_op = Some(sha.clone());
        }

        let repo = self.borrow_repo()?;
        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        if is_committing && !self.has_conflicts()? && self.has_staged_changes()? {
            let committer = repo.signature()?;
            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD has no target, failed to commit after cherrypick."),
            };
            self.git_commit(String::from(GitManager::get_utf8_string(commit.message(), "Commit Message")?), &commit.author(), &committer, vec![&head_commit])?;
        }

        Ok(())
    }

    pub fn git_revert(&mut self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha wasn't included in payload from the front-end."),
        };
        let is_committing = match json_hm.get("isCommitting") {
            Some(s) => s == "true",
            None => bail!("isCommitting wasn't included in payload from the front-end."),
        };

        // This closure allows self to be borrowed mutably for cleanup.
        {
            let repo = self.borrow_repo()?;
            let commit = repo.find_commit(Oid::from_str(sha)?)?;

            repo.revert(&commit, None)?;
        }

        if !self.has_conflicts()? {
            self.cleanup_state()?;
        } else {
            self.sha_from_commit_from_op = Some(sha.clone());
        }

        let repo = self.borrow_repo()?;
        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        if is_committing && !self.has_conflicts()? && self.has_staged_changes()? {
            let committer = repo.signature()?;
            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD has no target, failed to commit after revert."),
            };

            let mut new_full_message = String::from("Revert \"");
            new_full_message.push_str(GitManager::get_utf8_string(commit.summary(), "Commit Summary")?);
            new_full_message.push('"');
            let (message_without_summary, uses_crlf) = GitManager::get_message_without_summary(GitManager::get_utf8_string(commit.message(), "Commit Message")?);
            if uses_crlf {
                new_full_message.push_str("\r\n\r\n");
            } else {
                new_full_message.push_str("\n\n");
            }
            new_full_message.push_str(&*message_without_summary);

            self.git_commit(new_full_message, &commit.author(), &committer, vec![&head_commit])?;
        }

        Ok(())
    }

    pub fn git_abort(&mut self) -> Result<()> {
        // This closure allows self to be borrowed mutably for cleanup.
        {
            let repo = self.borrow_repo()?;

            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD doesn't have a target commit, cannot abort to HEAD"),
            };

            repo.reset(head_commit.as_object(), ResetType::Hard, None)?;
        }
        self.cleanup_state()?;

        Ok(())
    }

    pub fn git_continue_cherrypick(&mut self) -> Result<()> {
        if !self.has_conflicts()? {
            // This closure allows self to be borrowed mutably for cleanup.
            {
                let repo = self.borrow_repo()?;

                let head_commit = match repo.head()?.target() {
                    Some(oid) => repo.find_commit(oid)?,
                    None => bail!("HEAD doesn't have a target commit (which is where a cherrypick operation starts on), can't complete cherrypick."),
                };

                let commit_from_op = match &self.sha_from_commit_from_op {
                    Some(c) => repo.find_commit(Oid::from_str(c)?)?,
                    None => bail!("Original commit from cherrypick operation wasn't captured, can't complete cherrypick."),
                };

                // Note: using the current user as the author as well since they could've modified the original commit.
                let committer = repo.signature()?;

                let mut new_full_message = String::from("Revert \"");
                new_full_message.push_str(GitManager::get_utf8_string(commit_from_op.summary(), "Commit Summary")?);
                new_full_message.push('"');
                let (message_without_summary, uses_crlf) = GitManager::get_message_without_summary(GitManager::get_utf8_string(commit_from_op.message(), "Commit Message")?);
                if uses_crlf {
                    new_full_message.push_str("\r\n\r\n");
                } else {
                    new_full_message.push_str("\n\n");
                }
                new_full_message.push_str(&*message_without_summary);

                self.git_commit(new_full_message, &committer, &committer, vec![&head_commit])?;
            }
            self.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_continue_revert(&mut self) -> Result<()> {
        if !self.has_conflicts()? {
            // This closure allows self to be borrowed mutably for cleanup.
            {
                let repo = self.borrow_repo()?;

                let head_commit = match repo.head()?.target() {
                    Some(oid) => repo.find_commit(oid)?,
                    None => bail!("HEAD doesn't have a target commit (which is where a revert operation starts on), can't complete revert."),
                };

                let commit_from_op = match &self.sha_from_commit_from_op {
                    Some(c) => repo.find_commit(Oid::from_str(c)?)?,
                    None => bail!("Original commit from revert operation wasn't captured, can't complete revert."),
                };

                // Note: using the current user as the author as well since they could've modified the original commit.
                let committer = repo.signature()?;
                self.git_commit(String::from(GitManager::get_utf8_string(commit_from_op.message(), "Commit Message")?), &committer, &committer, vec![&head_commit])?;
            }
            self.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_continue_merge(&mut self) -> Result<()> {
        if !self.has_conflicts()? {
            // This closure allows self to be borrowed mutably for cleanup.
            {
                let repo = self.borrow_repo()?;

                let head_commit = match repo.head()?.target() {
                    Some(oid) => repo.find_commit(oid)?,
                    None => bail!("HEAD doesn't have a target commit (which is where a merge operation starts on), can't complete merge."),
                };

                let commit_from_op;
                let mut short_sha;
                match &self.sha_from_commit_from_op {
                    Some(c) => {
                        commit_from_op = repo.find_commit(Oid::from_str(c)?)?;
                        short_sha = c.clone();
                        short_sha.truncate(5);
                    },
                    None => bail!("Original commit from merge operation wasn't captured, can't complete merge."),
                };

                let mut message = String::from("Merge commit ");
                message.push_str(&*short_sha);
                message.push_str(" into commit ");
                let mut head_short_sha = head_commit.id().to_string();
                head_short_sha.truncate(5);
                message.push_str(&*head_short_sha);

                let committer = repo.signature()?;
                self.git_commit(message, &committer, &committer, vec![&head_commit, &commit_from_op])?;
            }
            self.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_abort_rebase(&self) -> Result<()> {
        let repo = self.borrow_repo()?;
        let mut rebase = repo.open_rebase(None)?;

        rebase.abort()?;
        Ok(())
    }

    pub fn git_continue_rebase(&self) -> Result<()> {
        let repo = self.borrow_repo()?;
        let mut rebase = repo.open_rebase(None)?;

        // Need to make sure there are no conflicts and we've committed before continuing.
        if self.has_conflicts()? || self.has_unstaged_changes()? {
            return Ok(());
        } else if self.has_staged_changes()? {
            rebase.commit(None, &repo.signature()?, None)?;
        }

        self.iterate_through_rebase(repo, &mut rebase)?;

        Ok(())
    }

    pub fn git_reset(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha wasn't included in payload from the front-end."),
        };
        let reset_type_string = match json_hm.get("type") {
            Some(s) => s,
            None => bail!("type wasn't included in payload from the front-end."),
        };

        let reset_type;
        if reset_type_string == "soft" {
            reset_type = ResetType::Soft;
        } else if reset_type_string == "mixed" {
            reset_type = ResetType::Mixed;
        } else if reset_type_string == "hard" {
            reset_type = ResetType::Hard;
        } else {
            bail!("type from front-end payload isn't a valid option. Choices are 'soft', 'mixed', or 'hard'");
        }

        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        repo.reset(commit.as_object(), reset_type, None)?;

        Ok(())
    }

    fn git_checkout(&self, local_ref: &Reference) -> Result<()> {
        let repo = self.borrow_repo()?;

        let local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;
        let commit = match local_ref.target() {
            Some(oid) => repo.find_commit(oid)?,
            None => bail!("Trying to check out branch that has no target commit."),
        };
        let tree = commit.tree()?;

        repo.checkout_tree(tree.as_object(), None)?;
        repo.set_head(local_full_name)?;
        Ok(())
    }

    pub fn git_checkout_from_json(&self, ref_full_name: &str) -> Result<()> {
        self.git_checkout(&self.borrow_repo()?.find_reference(ref_full_name)?)?;
        Ok(())
    }

    pub fn git_checkout_detached_head(&self, sha: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let oid = Oid::from_str(sha)?;
        let tree = repo.find_commit(oid)?.tree()?;

        repo.checkout_tree(tree.as_object(), None)?;
        repo.set_head_detached(oid)?;
        Ok(())
    }

    pub fn git_checkout_remote(&self, json_string: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_data: HashMap<String, String> = serde_json::from_str(json_string)?;
        let remote_branch_shortname = match json_data.get("branch_shorthand") {
            Some(n) => n,
            None => bail!("JSON Data is missing branch_shorthand attribute."),
        };
        let remote_branch_full_name = match json_data.get("full_branch_name") {
            Some(n) => n,
            None => bail!("JSON Data is missing full_branch_name attribute."),
        };

        // Look for a local branch that already exists for the specified remote branch. If one exists,
        // check it out instead.
        for local_b_result in repo.branches(Some(BranchType::Local))? {
            let (local_b, _) = local_b_result?;
            let local_upstream = match local_b.upstream() {
                Ok(b) => b,
                Err(_) => {
                    continue;
                },
            };
            let local_upstream_full_name = GitManager::get_utf8_string(local_upstream.get().name(), "Branch Name")?;
            if local_upstream_full_name == remote_branch_full_name {
                return self.git_checkout(local_b.get());
            }
        }

        // If there's no local branch, create a new one with the name used by the remote branch.
        let remote_branch_name_parts: Vec<&str> = remote_branch_shortname.split("/").collect();
        let mut local_branch_shortname = String::new();
        for i in 1..remote_branch_name_parts.len() {
            local_branch_shortname.push_str(remote_branch_name_parts[i]);
            if i < remote_branch_name_parts.len() - 1 {
                local_branch_shortname.push('/');
            }
        }
        let remote_branch = repo.find_branch(remote_branch_shortname, BranchType::Remote)?;
        let commit = match remote_branch.get().target() {
            Some(oid) => repo.find_commit(oid)?,
            None => bail!("Selected remote branch isn't targeting a commit, can't checkout!"),
        };
        let mut local_branch = repo.branch(&*local_branch_shortname, &commit, false)?;
        local_branch.set_upstream(Some(remote_branch_shortname))?;

        self.git_checkout(local_branch.get())
    }

    fn git_stage(&self, status: u8, path: &String) -> Result<()> {
        let repo = self.borrow_repo()?;

        let mut index = repo.index()?;
        if status == 2 {  // If file is deleted
            index.remove_path(path.as_ref())?;
        } else {
            index.add_path(path.as_ref())?;
        }
        index.write()?;

        Ok(())
    }

    pub fn git_stage_from_json(&self, json_string: &str) -> Result<()> {
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_string)?;

        self.git_stage(diff_delta.get_status(), diff_delta.get_path())?;

        Ok(())
    }

    pub fn git_unstage(&self, json_string: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_string)?;

        let mut index = repo.index()?;
        let status = diff_delta.get_status();
        if status == 2 || status == 3 {  // If file is deleted or modified
            let head_commit = match repo.head()?.target() {
                Some(oid) => {
                    repo.find_commit(oid)?
                },
                None => bail!("Head has no target commit"),
            };
            repo.reset_default(Some(head_commit.as_object()), [diff_delta.get_path()])?;
        } else {
            index.remove_path(diff_delta.get_path().as_ref())?;
        }
        index.write()?;

        Ok(())
    }

    fn set_diff_find_similar(diff: &mut Diff) -> Result<()> {
        let mut opts = DiffFindOptions::new();
        opts.renames(true);
        opts.copies(true);

        diff.find_similar(Some(&mut opts))?;
        Ok(())
    }

    pub fn get_unstaged_changes(&self) -> Result<Diff> {
        let repo = self.borrow_repo()?;

        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        diff_options.recurse_untracked_dirs(true);
        diff_options.show_untracked_content(true);

        let mut diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;
        GitManager::set_diff_find_similar(&mut diff)?;

        Ok(diff)
    }

    pub fn get_staged_changes(&self) -> Result<Diff> {
        let repo = self.borrow_repo()?;

        let head_ref = repo.head()?;
        let commit = match head_ref.target() {
            Some(oid) => Some(repo.find_commit(oid)?),
            None => None,
        };
        let tree = match commit {
            Some(c) => Some(c.tree()?),
            None => None,
        };

        let mut diff = repo.diff_tree_to_index(tree.as_ref(), None, None)?;
        GitManager::set_diff_find_similar(&mut diff)?;

        Ok(diff)
    }

    fn get_file_index_in_diff(diff: &Diff, path: &str) -> Result<usize> {
        let file_index_opt = diff.deltas().position(|dd| {
            match dd.new_file().path() {
                Some(p) => {
                    match p.to_str() {
                        Some(s) => path == s,
                        None => false,
                    }
                },
                None => false,
            }
        });
        match file_index_opt {
            Some(i) => Ok(i),
            None => bail!("Selected file not found."),
        }
    }

    pub fn get_file_diff(&self, json_str: &str) -> Result<FileInfo> {
        let repo = self.borrow_repo()?;
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let file_path = match json_hm.get("file_path") {
            Some(s) => s,
            None => bail!("file_path not returned from front-end payload."),
        };
        let change_type = match json_hm.get("change_type") {
            Some(s) => s,
            None => bail!("change_type not returned from front-end payload."),
        };
        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha not returned from front-end payload."),
        };

        let diff;
        if change_type == "unstaged" {
            diff = self.get_unstaged_changes()?;
        } else if change_type == "staged" {
            diff = self.get_staged_changes()?;
        } else if change_type == "commit" {
            let commit = repo.find_commit(Oid::from_str(sha)?)?;
            diff = get_commit_changes(&commit, &repo)?;
        } else {
            bail!("change_type not a valid type. Needs to be 'staged', 'unstaged', or 'commit'");
        }

        let file_index = GitManager::get_file_index_in_diff(&diff, file_path.as_str())?;

        let patch_opt = Patch::from_diff(&diff, file_index)?;
        let mut file_lines = vec![];
        let file_type = String::from(file_path.split(".").last().unwrap_or(""));
        match patch_opt {
            Some(patch) => {
                for i in 0..patch.num_hunks() {
                    let line_count = patch.num_lines_in_hunk(i)?;
                    for j in 0..line_count {
                        let line = patch.line_in_hunk(i, j)?;
                        file_lines.push(FileLineInfo::from_diff_line(line, &file_type)?);
                    }
                }
            },
            None => bail!("Patch not found in diff."),
        }

        let file_info = FileInfo::new(change_type.clone(), file_lines);
        Ok(file_info)
    }

    pub fn git_stage_all(&self) -> Result<()> {
        let repo = self.borrow_repo()?;

        let mut index = repo.index()?;
        index.add_all(&["."], IndexAddOption::DEFAULT, None)?;
        index.write()?;

        Ok(())
    }

    // This is used for performing commits in rebases, merges, cherrypicks, and reverts
    fn git_commit(&self, full_message: String, author: &Signature, committer: &Signature, parent_commits: Vec<&Commit>) -> Result<()> {
        let repo = self.borrow_repo()?;

        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        index.write()?;
        let tree = repo.find_tree(tree_oid)?;

        repo.commit(Some("HEAD"), author, committer, &*full_message, &tree, parent_commits.as_slice())?;

        Ok(())
    }

    pub fn git_commit_from_json(&self, json_string: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        // TODO: Add way to set signature in git config
        let signature = repo.signature()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_string)?;
        let summary = match json_hm.get("summaryText") {
            Some(s) => s,
            None => bail!("Front-end payload did not include summaryText"),
        };
        let message = match json_hm.get("messageText") {
            Some(s) => s,
            None => bail!("Front-end payload did not include messageText"),
        };

        let mut full_message = summary.clone();
        if message != "" {
            full_message.push_str("\n\n");
            full_message.push_str(message);
        }

        let mut parents = vec![];
        let commit;
        if let Some(oid) = repo.head()?.target() {
            commit = repo.find_commit(oid)?;
            parents.push(&commit);
        }

        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        index.write()?;
        let tree = repo.find_tree(tree_oid)?;

        repo.commit(Some("HEAD"), &signature, &signature, &*full_message, &tree, parents.as_slice())?;
        Ok(())
    }

    pub fn git_discard_changes(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let path = match json_hm.get("path") {
            Some(s) => s,
            None => bail!("path was not included in the payload from the front-end"),
        };
        let change_type = match json_hm.get("change_type") {
            Some(s) => s,
            None => bail!("change_type was not included in the payload from the front-end"),
        };
        let status: u8 = match json_hm.get("status") {
            Some(s) => s.parse()?,
            None => bail!("status was not included in the payload from the front-end"),
        };

        let mut cb = CheckoutBuilder::new();
        cb.path(path);
        cb.force();

        if change_type == "unstaged" && status == 7 {  // if unstaged and untracked need to stage it to discard.
            self.git_stage(status, path)?;
        } else if status == 4 {  // if renamed, need to discard the new file and old file.
            let diff;
            if change_type == "unstaged" {
                diff = self.get_unstaged_changes()?;
            } else if change_type == "staged" {
                diff = self.get_staged_changes()?;
            } else {
                bail!("Attempting to discard a renamed file that's neither staged nor unstaged. This error should technically be impossible.");
            }

            let diff_delta = match diff.get_delta(GitManager::get_file_index_in_diff(&diff, path.as_str())?) {
                Some(dd) => dd,
                None => bail!("Couldn't find DiffDelta associated with the renamed file, unable to discard changes."),
            };

            let old_path = match diff_delta.old_file().path() {
                Some(p) => p,
                None => bail!("Couldn't find old file path of renamed file, unable to discard changes."),
            };

            cb.path(old_path);
        }

        repo.checkout_head(Some(&mut cb))?;

        Ok(())
    }

    pub fn git_delete_local_branch(&self, branch_shorthand: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        let mut branch = repo.find_branch(branch_shorthand, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    pub fn git_delete_remote_branch(&self, branch_shorthand: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let remote_branch = repo.find_branch(branch_shorthand, BranchType::Remote)?;
        let remote_branch_full_name = GitManager::get_utf8_string(remote_branch.get().name(), "Branch Name")?;

        // Look for a local branch that already exists for the specified remote branch. If one exists,
        // unset its upstream.
        for local_b_result in repo.branches(Some(BranchType::Local))? {
            let (mut local_b, _) = local_b_result?;
            let local_upstream = match local_b.upstream() {
                Ok(b) => b,
                Err(_) => {
                    continue;
                },
            };
            let local_upstream_full_name = GitManager::get_utf8_string(local_upstream.get().name(), "Branch Name")?;
            if local_upstream_full_name == remote_branch_full_name {
                local_b.set_upstream(None)?;
            }
        }

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.get_remote_callbacks());

        let mut sb = String::from(":refs/heads/");
        let first_slash_index = match branch_shorthand.find("/") {
            Some(i) => i,
            None => bail!("Remote Branch doesn't seem to have a remote in its name?"),
        };
        let mut remote = repo.find_remote(&branch_shorthand[0..first_slash_index])?;
        sb.push_str(&branch_shorthand[(first_slash_index + 1)..]);
        remote.push(&[sb.as_str()], Some(&mut push_options))?;
        Ok(())
    }

    pub fn git_delete_tag(&self, tag_name: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        repo.tag_delete(tag_name)?;
        Ok(())
    }

    pub fn git_fetch(&self) -> Result<()> {
        let repo = self.borrow_repo()?;
        let remote_string_array = repo.remotes()?;
        let empty_refspecs: &[String] = &[];
        for remote_string_opt in remote_string_array.iter() {
            let remote_string = GitManager::get_utf8_string(remote_string_opt, "Remote Name")?;
            let mut remote = repo.find_remote(remote_string)?;
            let mut fetch_options = FetchOptions::new();
            fetch_options.download_tags(AutotagOption::All);
            fetch_options.prune(FetchPrune::On);
            fetch_options.remote_callbacks(self.get_remote_callbacks());
            remote.fetch(empty_refspecs, Some(&mut fetch_options), None)?;
        }
        Ok(())
    }

    pub fn git_pull(&self) -> Result<()> {
        let repo = self.borrow_repo()?;

        // Fetch first to make sure everything's up to date.
        self.git_fetch()?;

        let mut local_ref = repo.head()?;
        let local_shorthand = GitManager::get_utf8_string(local_ref.shorthand(), "Branch Name")?;
        let local_branch = repo.find_branch(local_shorthand, BranchType::Local)?;

        let remote_branch = local_branch.upstream()?;
        let remote_ref = remote_branch.get();
        let remote_target = match remote_ref.target() {
            Some(oid) => oid,
            None => bail!("Remote branch is not targeting a commit, cannot pull."),
        };
        let remote_ac = repo.find_annotated_commit(remote_target)?;

        let (ma, mp) = repo.merge_analysis(&[&remote_ac])?;

        if ma.is_none() {
            bail!("Merge analysis indicates no merge is possible. If you're reading this, your repository may be corrupted.");
        } else if ma.is_unborn() {
            bail!("The HEAD of the current repository is “unborn” and does not point to a valid commit. No pull can be performed, but the caller may wish to simply set HEAD to the target commit(s).");
        } else if ma.is_up_to_date() {
            return Ok(());
        } else if ma.is_fast_forward() && !mp.is_no_fast_forward() {
            println!("Performing fast forward merge for pull!");
            let commit = match remote_ref.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("Trying to check out branch that has no target commit."),
            };
            let tree = commit.tree()?;
            repo.checkout_tree(tree.as_object(), None)?;
            local_ref.set_target(remote_target, "oxidized_git pull: setting new target for local ref")?;
            return Ok(());
        } else if ma.is_normal() && !mp.is_fastforward_only() {
            println!("Performing rebase for pull!");
            let mut rebase = repo.rebase(None, None, Some(&remote_ac), None)?;
            let mut has_conflicts = false;
            for step in rebase.by_ref() {
                step?;
                if self.has_conflicts()? || self.has_unstaged_changes()? || self.has_staged_changes()? {
                    has_conflicts = true;
                    break;
                }
            }
            if has_conflicts {
                rebase.abort()?;
                bail!("Pull by rebase aborted because changes on local branch differ from remote branch!");
            }
            rebase.finish(None)?;
            return Ok(());
        } else if (ma.is_fast_forward() && mp.is_no_fast_forward()) || (ma.is_normal() && mp.is_fastforward_only()) {
            bail!("It looks like a pull may be possible, but your MergePreference(s) are preventing it. If you have --no-ff AND/OR --ff-only enabled, consider disabling one or both.");
        }
        bail!("Merge analysis failed to make any determination on how to proceed with the pull. If you're reading this, your repository may be corrupted.")
    }

    pub fn git_push(&self, push_options_json_opt: Option<&str>) -> Result<()> {
        let repo = self.borrow_repo()?;

        let is_force;
        let remote_name_from_frontend_opt;
        let push_options: HashMap<String, String>;
        if let Some(push_options_json) = push_options_json_opt {
            push_options = serde_json::from_str(push_options_json)?;
            is_force = match push_options.get("isForcePush") {
                Some(s) => s == "true",
                None => bail!("isForcePush not included in payload from front-end."),
            };
            remote_name_from_frontend_opt = match push_options.get("selectedRemote") {
                Some(s) => Some(s.as_str()),
                None => bail!("selectedRemote not included in payload from front-end."),
            };
        } else {
            is_force = false;
            remote_name_from_frontend_opt = None;
        }

        let local_ref = repo.head()?;
        let local_full_name = GitManager::get_utf8_string(local_ref.name(), "Branch Name")?;

        let mut is_creating_new_remote_branch = false;
        let mut remote = match repo.branch_upstream_remote(local_full_name) {
            Ok(b) => {
                let remote_name = GitManager::get_utf8_string(b.as_str(), "Remote Name")?;
                repo.find_remote(remote_name)?
            },
            Err(_e) => {
                is_creating_new_remote_branch = true;
                match remote_name_from_frontend_opt {
                    Some(rn) => repo.find_remote(rn)?,
                    None => bail!("Attempted to push with no upstream branch and no specified remote."),
                }
            },
        };

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.get_remote_callbacks());

        let mut sb = String::from(local_full_name);
        if is_force {
            sb.insert(0, '+');
        }

        remote.push(&[sb.as_str()], Some(&mut push_options))?;

        if is_creating_new_remote_branch {
            let local_branch_shorthand = GitManager::get_utf8_string(local_ref.shorthand(), "Branch Name")?;
            let remote_name = GitManager::get_utf8_string(remote.name(), "Remote Name")?;
            let new_remote_branch_shorthand = format!("{remote_name}/{local_branch_shorthand}");
            let mut local_branch = repo.find_branch(local_branch_shorthand, BranchType::Local)?;
            local_branch.set_upstream(Some(&*new_remote_branch_shorthand))?;
        }

        Ok(())
    }

    pub fn git_branch(&self, json_string: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let branch_options: HashMap<String, String> = serde_json::from_str(json_string)?;
        let branch_name = match branch_options.get("branch_name") {
            Some(s) => s,
            None => bail!("branch_name not included in payload from front-end."),
        };
        let checkout_on_create = match branch_options.get("checkout_on_create") {
            Some(s) => s == "true",
            None => bail!("checkout_on_create not included in payload from front-end."),
        };

        let target_commit = match repo.head()?.target() {
            Some(oid) => repo.find_commit(oid)?,
            None => bail!("Current head not pointing at commit, cannot create branch."),
        };

        let new_branch = repo.branch(branch_name, &target_commit, false)?;

        if checkout_on_create {
            self.git_checkout(new_branch.get())?;
        }

        Ok(())
    }

    #[allow(unused_unsafe)]
    fn get_remote_callbacks(&self) -> RemoteCallbacks {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            let username;
            let pass;
            unsafe {
                username = match keytar::get_password("oxidized_git", "username") {
                    Ok(p) => p,
                    Err(_) => return Err(git2::Error::from_str("Error finding username in keychain!")),
                };
                pass = match keytar::get_password("oxidized_git", "password") {
                    Ok(p) => p,
                    Err(_) => return Err(git2::Error::from_str("Error finding password in keychain!")),
                };
            }
            if username.success && pass.success {
                Cred::userpass_plaintext(&*username.password, &*pass.password)
            } else {
                Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials"))
            }
        });
        callbacks.push_update_reference(|_ref_name, status_msg| {
            match status_msg {
                Some(m) => Err(git2::Error::from_str(&*format!("Error(s) during push: {}", m))),
                None => Ok(()),
            }
        });
        callbacks
    }

    #[allow(unused_unsafe)]
    pub fn set_credentials(&self, credentials_json_string: &str) -> Result<()> {
        let credentials_json: HashMap<String, String> = serde_json::from_str(credentials_json_string)?;
        let username = match credentials_json.get("username") {
            Some(u) => u,
            None => bail!("No username supplied"),
        };
        let password = match credentials_json.get("password") {
            Some(p) => p,
            None => bail!("No password supplied"),
        };

        unsafe {
            keytar::set_password("oxidized_git", "username", username)?;
            keytar::set_password("oxidized_git", "password", password)?;
        }

        Ok(())
    }
}

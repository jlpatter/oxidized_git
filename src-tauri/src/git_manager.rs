use std::collections::HashMap;
use std::path::PathBuf;
use std::str;
use anyhow::{bail, Result};
use directories::BaseDirs;
use git2::{AutotagOption, BranchType, Commit, Cred, Delta, Diff, DiffFindOptions, DiffLine, DiffOptions, FetchOptions, FetchPrune, Oid, Patch, PushOptions, Reference, RemoteCallbacks, Repository, ResetType, Signature, Sort};
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

pub struct GitManager {
    repo: Option<Repository>,
    sha_from_commit_from_op: Option<String>,
}

impl GitManager {
    pub const fn new() -> Self {
        Self {
            repo: None,
            sha_from_commit_from_op: None,
        }
    }

    pub fn get_utf8_string<'a, 'b>(value: Option<&'a str>, str_name_type: &'b str) -> Result<&'a str> {
        match value {
            Some(n) => Ok(n),
            None => bail!(format!("{} uses invalid utf-8!", str_name_type)),
        }
    }

    pub fn get_repo(&self) -> Result<&Repository> {
        let repo_temp_opt = &self.repo;
        match repo_temp_opt {
            Some(repo) => Ok(repo),
            None => bail!("No repo loaded to perform operation on."),
        }
    }

    pub fn has_open_repo(&self) -> bool {
        match self.get_repo() {
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
        let repo = self.get_repo()?;
        repo.cleanup_state()?;
        self.sha_from_commit_from_op = None;
        Ok(())
    }

    pub fn git_revwalk(&self) -> Result<Vec<Oid>> {
        let repo = self.get_repo()?;
        let mut revwalk = repo.revwalk()?;
        let mut oid_vec: Vec<Oid> = vec![];
        for branch_result in repo.branches(None)? {
            let (branch, _) = branch_result?;
            match branch.get().target() {
                Some(oid) => oid_vec.push(oid),
                None => (),
            };
        };
        // Sort Oids by date first
        oid_vec.sort_by(|a, b| {
            repo.find_commit(*b).unwrap().time().seconds().partial_cmp(&repo.find_commit(*a).unwrap().time().seconds()).unwrap()
        });
        for oid in oid_vec {
            revwalk.push(oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;

        let preferences = config_manager::get_preferences()?;
        let limit_commits = preferences.get_limit_commits();
        let commit_count = preferences.get_commit_count();

        let mut oid_list: Vec<Oid> = vec![];
        for (i, commit_oid_result) in revwalk.enumerate() {
            if limit_commits && i >= commit_count {
                break;
            }
            oid_list.push(commit_oid_result?);
        }
        Ok(oid_list)
    }

    pub fn get_ref_from_name(&self, ref_full_name: &str) -> Result<Reference> {
        Ok(self.get_repo()?.find_reference(ref_full_name)?)
    }

    pub fn get_commit_info(&self, sha: &str) -> Result<CommitInfo> {
        let repo = self.get_repo()?;

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

    fn has_staged_changes(&self) -> Result<bool> {
        let diff = self.get_staged_changes()?;

        if diff.stats()?.files_changed() > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
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
            let repo = self.get_repo()?;
            let commit = repo.find_commit(Oid::from_str(sha)?)?;

            repo.cherrypick(&commit, None)?;
        }

        if !self.has_conflicts()? {
            self.cleanup_state()?;
        } else {
            self.sha_from_commit_from_op = Some(sha.clone());
        }

        let repo = self.get_repo()?;
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
            let repo = self.get_repo()?;
            let commit = repo.find_commit(Oid::from_str(sha)?)?;

            repo.revert(&commit, None)?;
        }

        if !self.has_conflicts()? {
            self.cleanup_state()?;
        } else {
            self.sha_from_commit_from_op = Some(sha.clone());
        }

        let repo = self.get_repo()?;
        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        if is_committing && !self.has_conflicts()? && self.has_staged_changes()? {
            let committer = repo.signature()?;
            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD has no target, failed to commit after revert."),
            };
            self.git_commit(String::from(GitManager::get_utf8_string(commit.message(), "Commit Message")?), &commit.author(), &committer, vec![&head_commit])?;
        }

        Ok(())
    }

    pub fn git_abort(&mut self) -> Result<()> {
        // This closure allows self to be borrowed mutably for cleanup.
        {
            let repo = self.get_repo()?;

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
                let repo = self.get_repo()?;

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
                self.git_commit(String::from(GitManager::get_utf8_string(commit_from_op.message(), "Commit Message")?), &committer, &committer, vec![&head_commit])?;
            }
            self.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_continue_revert(&mut self) -> Result<()> {
        if !self.has_conflicts()? {
            // This closure allows self to be borrowed mutably for cleanup.
            {
                let repo = self.get_repo()?;

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

    pub fn git_reset(&self, json_str: &str) -> Result<()> {
        let repo = self.get_repo()?;

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

    pub fn git_checkout(&self, local_ref: &Reference) -> Result<()> {
        let repo = self.get_repo()?;

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

    pub fn git_checkout_remote(&self, json_string: &str) -> Result<()> {
        let repo = self.get_repo()?;

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

    pub fn git_stage(&self, json_string: &str) -> Result<()> {
        let repo = self.get_repo()?;
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_string)?;

        let mut index = repo.index()?;
        if diff_delta.get_status() == 2 {  // If file is deleted
            index.remove_path(diff_delta.get_path().as_ref())?;
        } else {
            index.add_path(diff_delta.get_path().as_ref())?;
        }
        index.write()?;

        Ok(())
    }

    pub fn git_unstage(&self, json_string: &str) -> Result<()> {
        let repo = self.get_repo()?;
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
        let repo = self.get_repo()?;

        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        diff_options.recurse_untracked_dirs(true);
        diff_options.show_untracked_content(true);

        let mut diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;
        GitManager::set_diff_find_similar(&mut diff)?;

        Ok(diff)
    }

    pub fn get_staged_changes(&self) -> Result<Diff> {
        let repo = self.get_repo()?;

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

    pub fn get_file_diff(&self, json_str: &str) -> Result<FileInfo> {
        let repo = self.get_repo()?;
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

        let file_index_opt = diff.deltas().position(|dd| {
            match dd.new_file().path() {
                Some(p) => {
                    match p.to_str() {
                        Some(s) => file_path.as_str() == s,
                        None => false,
                    }
                },
                None => false,
            }
        });
        let file_index = match file_index_opt {
            Some(i) => i,
            None => bail!("Selected file not found. This shouldn't happen since this uses the same methods that are used to generate the file list."),
        };

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

    // This is used for performing commits in rebases, merges, cherrypicks, and reverts
    fn git_commit(&self, full_message: String, author: &Signature, committer: &Signature, parent_commits: Vec<&Commit>) -> Result<()> {
        let repo = self.get_repo()?;

        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        index.write()?;
        let tree = repo.find_tree(tree_oid)?;

        repo.commit(Some("HEAD"), author, committer, &*full_message, &tree, parent_commits.as_slice())?;

        Ok(())
    }

    pub fn git_commit_from_json(&self, json_string: &str) -> Result<()> {
        let repo = self.get_repo()?;
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

    pub fn git_delete_local_branch(&self, branch_shorthand: &str) -> Result<()> {
        let repo = self.get_repo()?;
        let mut branch = repo.find_branch(branch_shorthand, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    pub fn git_delete_remote_branch(&self, branch_shorthand: &str) -> Result<()> {
        let repo = self.get_repo()?;

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
        let repo = self.get_repo()?;
        repo.tag_delete(tag_name)?;
        Ok(())
    }

    pub fn git_fetch(&self) -> Result<()> {
        let repo = self.get_repo()?;
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
        let repo = self.get_repo()?;

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
                let diff = repo.diff_index_to_workdir(None, None)?;
                if diff.stats()?.files_changed() > 0 {
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
        let repo = self.get_repo()?;

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
        let repo = self.get_repo()?;

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

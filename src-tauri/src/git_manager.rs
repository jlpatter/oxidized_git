use std::collections::{HashMap, VecDeque};
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use std::{fs, str};
use anyhow::{bail, Result};
use git2::{AutotagOption, Branch, BranchType, Commit, Cred, Delta, Diff, DiffFindOptions, DiffLine, DiffLineType, DiffOptions, ErrorCode, FetchOptions, FetchPrune, IndexAddOption, ObjectType, Oid, Patch, PushOptions, Rebase, Reference, RemoteCallbacks, Repository, RepositoryState, ResetType, Signature, Sort, StashFlags};
use git2::build::{CheckoutBuilder, RepoBuilder};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
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

fn get_content_from_diff_line(diff_line: &DiffLine) -> Result<String> {
    let mut content_string = String::from(str::from_utf8(diff_line.content())?);
    trim_newline(&mut content_string);
    content_string = html_escape::encode_text(&content_string).parse()?;
    Ok(content_string)
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
        let content_string = get_content_from_diff_line(&diff_line)?;
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

#[derive(Clone)]
pub enum LineInfo {
    SomeFileLineInfo(FileLineInfo),
    SomeSeparator(String),
}

impl Serialize for LineInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            LineInfo::SomeFileLineInfo(f) => f.serialize(serializer),
            LineInfo::SomeSeparator(s) => s.serialize(serializer),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct FileInfo {
    change_type: String,
    file_lines: Vec<LineInfo>,
}

impl FileInfo {
    pub fn new(change_type: String, file_lines: Vec<LineInfo>) -> Self {
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

#[derive(Clone, Serialize)]
pub struct InteractiveRebaseInfo {
    onto_sha: String,
    commits: Vec<HashMap<String, String>>,
}

impl InteractiveRebaseInfo {
    pub fn new(onto_sha: String, commits: Vec<HashMap<String, String>>) -> Self {
        Self {
            onto_sha,
            commits,
        }
    }
}

#[derive(Deserialize)]
pub struct InteractiveRebaseOps {
    onto_sha: String,
    commit_ops: Vec<HashMap<String, String>>,
}

impl InteractiveRebaseOps {
    pub fn clone_onto_sha(&self) -> String {
        self.onto_sha.clone()
    }

    pub fn clone_commit_ops(&self) -> Vec<HashMap<String, String>> {
        self.commit_ops.clone()
    }
}

fn get_commit_changes<'a, 'b>(commit: &'a Commit, repo: &'b Repository) -> Result<Diff<'b>> {
    let commit_tree = commit.tree()?;

    let mut diff_opt = None;
    for parent_commit in commit.parents() {
        let mut diff = repo.diff_tree_to_tree(Some(&parent_commit.tree()?), Some(&commit_tree), None)?;
        GitManager::set_diff_find_similar(&mut diff)?;
        // For merge commits, the diff between a merge commit and the parent from the branch that was merged will be empty,
        // so find the diff that's populated.
        if diff.stats()?.files_changed() > 0 {
            return Ok(diff);
        }
        diff_opt = Some(diff);
    }

    // If there are parents but no diff was returned earlier, then the commit is empty and should return the empty diff.
    if let Some(d) = diff_opt {
        return Ok(d);
    }

    // If there are no parents, get the diff between this commit and nothing.
    let mut diff = repo.diff_tree_to_tree(None, Some(&commit_tree), None)?;
    GitManager::set_diff_find_similar(&mut diff)?;

    Ok(diff)
}

pub struct GitManager {
    repo: Option<Repository>,
    old_graph_starting_shas: Vec<String>,
}

impl GitManager {
    pub const fn new() -> Self {
        Self {
            repo: None,
            old_graph_starting_shas: vec![],
        }
    }

    pub fn get_utf8_string<'a, 'b>(value: Option<&'a str>, str_name_type: &'b str) -> Result<&'a str> {
        match value {
            Some(n) => Ok(n),
            None => bail!(format!("{} uses invalid utf-8!", str_name_type)),
        }
    }

    fn get_string_from_serde_string(value: Option<&str>) -> Result<&str> {
        match value {
            Some(s) => Ok(s),
            None => bail!("Invalid JSON String!"),
        }
    }

    pub fn borrow_repo(&self) -> Result<&Repository> {
        let repo_temp_opt = &self.repo;
        match repo_temp_opt {
            Some(repo) => Ok(repo),
            None => bail!("No repo loaded to perform operation on."),
        }
    }

    pub fn borrow_repo_mut(&mut self) -> Result<&mut Repository> {
        let repo_temp_opt = &mut self.repo;
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

    pub fn init_repo(&mut self, json_str: &str) -> Result<()> {
        let path_value: Value = serde_json::from_str(json_str)?;
        let path_str = GitManager::get_string_from_serde_string(path_value.as_str())?;
        self.repo = Some(Repository::init(Path::new(path_str))?);
        Ok(())
    }

    pub fn open_repo(&mut self, json_str: &str) -> Result<()> {
        let path_value: Value = serde_json::from_str(json_str)?;
        let path_str = GitManager::get_string_from_serde_string(path_value.as_str())?;
        self.repo = Some(Repository::open(Path::new(path_str))?);
        Ok(())
    }

    pub fn clone_repo(&mut self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let clone_url = match json_hm.get("clone_url") {
            Some(s) => s,
            None => bail!("clone_url not included in payload from the front-end"),
        };
        let clone_path = match json_hm.get("clone_path") {
            Some(s) => s,
            None => bail!("clone_path not included in payload from the front-end"),
        };

        let callbacks = GitManager::get_remote_callbacks();
        let mut fetch_options = FetchOptions::new();
        fetch_options.download_tags(AutotagOption::All);
        fetch_options.remote_callbacks(callbacks);

        let mut repo_builder = RepoBuilder::new();
        repo_builder.fetch_options(fetch_options);

        let project_name = match clone_url.split("/").last() {
            Some(s) => {
                &s[..(s.len() - 4)]
            },
            None => bail!("Clone url was empty?"),
        };

        let mut path_buf = PathBuf::from(clone_path);
        path_buf.push(project_name);

        create_dir_all(path_buf.as_path())?;

        self.repo = Some(repo_builder.clone(clone_url, path_buf.as_path())?);

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

    pub fn git_revwalk(&mut self, force_refresh: bool) -> Result<Option<Vec<Oid>>> {
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

        if force_refresh {
            self.old_graph_starting_shas = vec![];
        }

        if self.old_shas_eq_sorted_new_oids(&oid_vec) {
            return Ok(None);
        }

        // If you've reached here, the old and new starting oids are different. Update the old and perform the revwalk.
        self.old_graph_starting_shas = oid_vec.iter().map(|new_oid| {
            new_oid.to_string()
        }).collect();

        let repo = self.borrow_repo()?;
        let mut revwalk = repo.revwalk()?;

        for oid in oid_vec {
            revwalk.push(oid)?;
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;

        let preferences = config_manager::get_config()?;
        let limit_commits = match preferences.borrow_limit_commits() {
            Some(b) => b.clone(),
            None => bail!("limit_commits not present in config file!"),
        };
        let commit_count = match preferences.borrow_commit_count() {
            Some(i) => i.clone(),
            None => bail!("commit_count not present in config file!"),
        };

        let mut oid_list: Vec<Oid> = vec![];
        for (i, commit_oid_result) in revwalk.enumerate() {
            if limit_commits && i >= commit_count {
                break;
            }
            oid_list.push(commit_oid_result?);
        }
        Ok(Some(oid_list))
    }

    pub fn get_commit_info(&self, json_str: &str) -> Result<CommitInfo> {
        let sha_value: Value = serde_json::from_str(json_str)?;
        let sha: &str = GitManager::get_string_from_serde_string(sha_value.as_str())?;
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

    pub fn git_merge(&self, json_str: &str) -> Result<()> {
        let sha_value: Value = serde_json::from_str(json_str)?;
        let sha: &str = GitManager::get_string_from_serde_string(sha_value.as_str())?;
        let repo = self.borrow_repo()?;
        let annotated_commit = repo.find_annotated_commit(Oid::from_str(sha)?)?;

        repo.merge(&[&annotated_commit], None, None)?;

        if !self.has_conflicts()? {
            if self.has_staged_changes()? {
                let head_commit = match repo.head()?.target() {
                    Some(oid) => repo.find_commit(oid)?,
                    None => bail!("HEAD has no target, failed to commit after merging. It should fail earlier than this since there'd be no HEAD to merge into."),
                };
                let merge_parent_two = repo.find_commit(Oid::from_str(sha)?)?;
                let parent_commits = vec![&head_commit, &merge_parent_two];
                let committer = repo.signature()?;

                let mut short_sha = String::from(sha);
                short_sha.truncate(5);
                let mut head_short_sha = head_commit.id().to_string();
                head_short_sha.truncate(5);
                let message = String::from("Merge commit ") + short_sha.as_str() + " into commit " + head_short_sha.as_str();

                self.git_commit(message, &committer, &committer, parent_commits)?;

                repo.cleanup_state()?;
            } else {
                bail!("Merge commit had no changes, not really sure what would cause this...");
            }
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

    pub fn git_rebase(&self, json_str: &str) -> Result<()> {
        let sha_value: Value = serde_json::from_str(json_str)?;
        let sha: &str = GitManager::get_string_from_serde_string(sha_value.as_str())?;
        let repo = self.borrow_repo()?;
        let annotated_commit = repo.find_annotated_commit(Oid::from_str(sha)?)?;
        let mut rebase = repo.rebase(None, None, Some(&annotated_commit), None)?;

        self.iterate_through_rebase(repo, &mut rebase)?;

        Ok(())
    }

    pub fn git_begin_rebase_interactive(&self, json_str: &str) -> Result<InteractiveRebaseInfo> {
        let repo = self.borrow_repo()?;
        let sha_value: Value = serde_json::from_str(json_str)?;
        let sha: &str = GitManager::get_string_from_serde_string(sha_value.as_str())?;
        let annotated_commit = repo.find_annotated_commit(Oid::from_str(sha)?)?;

        // This is a hacky way of figuring out what commits need to be rebased:
        // this initializes a regular rebase and digs into the `.git` folder to find the commits
        // e.g. "project-root/.git/rebase-merge/cmt.1"
        let mut rebase = repo.rebase(None, None, Some(&annotated_commit), None)?;

        let mut rebase_path = repo.path().to_path_buf();
        rebase_path.push("rebase-merge");

        let mut commit_file_paths: Vec<PathBuf> = vec![];
        let paths = fs::read_dir(rebase_path)?;
        for path in paths {
            let path_buf = path?.path();
            let path_string = match path_buf.to_str() {
                Some(s) => s,
                None => bail!("Path to .git folder is not valid unicode!"),
            };

            if path_string.contains("cmt.") {
                commit_file_paths.push(path_buf);
            }
        }
        // Sort in reverse so we can render the list to the user.
        commit_file_paths.sort_by(|a, b| b.cmp(a));

        let mut parseable_commits = vec![];
        for path_buf in commit_file_paths {
            let mut sha = fs::read_to_string(path_buf)?;
            sha = String::from(sha.as_str().trim());

            let mut parseable_commit = HashMap::new();
            parseable_commit.insert(String::from("sha"), sha.clone());

            let oid = Oid::from_str(sha.as_str())?;
            let commit = repo.find_commit(oid)?;
            let summary = String::from(GitManager::get_utf8_string(commit.summary(), "Commit Summary")?);

            parseable_commit.insert(String::from("summary"), summary);

            parseable_commits.push(parseable_commit);
        }

        // Abort the regular rebase so we can perform our own operations on the commits
        // for an interactive rebase.
        rebase.abort()?;

        Ok(InteractiveRebaseInfo::new(String::from(sha), parseable_commits))
    }

    pub fn git_rebase_interactive(&self, json_str: &str) -> Result<()> {
        // TODO: May need to wrap this function in a function that aborts the rebase in progress
        // in case something goes wrong.
        let repo = self.borrow_repo()?;
        let ir_ops: InteractiveRebaseOps = serde_json::from_str(json_str)?;
        let onto_sha = ir_ops.clone_onto_sha();
        let commit_ops = ir_ops.clone_commit_ops();

        if repo.state() != RepositoryState::Clean {
            bail!("Repository not in a clean state (another operation is in progress). \
            Please complete or abort your previous operation before performing an interactive rebase.")
        }

        let head_ref = repo.head()?;
        let head_full_name = GitManager::get_utf8_string(head_ref.name(), "Branch Name")?;
        let orig_head_commit = match head_ref.target() {
            Some(oid) => repo.find_commit(oid)?,
            None => bail!("Current HEAD has no commit."),
        };

        let dot_git_path = repo.path().to_path_buf();
        let rebase_path = dot_git_path.join("rebase-merge");

        if rebase_path.exists() {
            bail!("Rebase metadata already present. Maybe another rebase is already in progress?");
        }

        let mut msg_num: usize = 0;

        // WARNING: ENTERING DANGER ZONE!!
        // Create files in .git to initialize an interactive rebase.

        // TODO: AUTO_MERGE
        // TODO: Might not need to set HEAD directly, should try rust-git2 function instead.
        fs::write(dot_git_path.join("HEAD"), onto_sha.clone())?;
        // TODO: MERGE_MSG

        // TODO: logs/HEAD

        create_dir_all(rebase_path.clone())?;
        // TODO: author-script
        File::create(rebase_path.join("done"))?;
        fs::write(rebase_path.join("end"), commit_ops.len().to_string())?;
        // TODO: git-rebase-todo
        // TODO: git-rebase-todo.backup
        fs::write(rebase_path.join("head-name"), head_full_name)?;
        File::create(rebase_path.join("interactive"))?;
        fs::write(rebase_path.join("msgnum"), msg_num.to_string())?;
        File::create(rebase_path.join("no-reschedule-failed-exec"))?;
        fs::write(rebase_path.join("onto"), onto_sha.clone())?;
        fs::write(rebase_path.join("orig-head"), orig_head_commit.id())?;
        // TODO: patch

        // Perform sequence in interactive rebase.

        for commit_op in commit_ops {
            let commit_op_sha = match commit_op.get("sha") {
                Some(s) => s,
                None => bail!("Payload from front-end is missing SHAs."),
            };

            let commit = repo.find_commit(Oid::from_str(commit_op_sha.as_str())?)?;
            let commit_message = GitManager::get_utf8_string(commit.message(), "Commit Message")?;

            fs::write(dot_git_path.join("REBASE_HEAD"), commit_op_sha.clone())?;
            fs::write(rebase_path.join("message"), commit_message)?;
            msg_num += 1;
            fs::write(rebase_path.join("msgnum"), msg_num.to_string())?;
            // TODO: stopped-sha - if rebase stops?
        }

        Ok(())
    }

    pub fn git_cherrypick(&self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha wasn't included in payload from the front-end."),
        };
        let is_committing = match json_hm.get("isCommitting") {
            Some(s) => s == "true",
            None => bail!("isCommitting wasn't included in payload from the front-end."),
        };

        let repo = self.borrow_repo()?;
        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        repo.cherrypick(&commit, None)?;

        if !self.has_conflicts()? {
            repo.cleanup_state()?;
        }

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

    pub fn git_revert(&self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;

        let sha = match json_hm.get("sha") {
            Some(s) => s,
            None => bail!("sha wasn't included in payload from the front-end."),
        };
        let is_committing = match json_hm.get("isCommitting") {
            Some(s) => s == "true",
            None => bail!("isCommitting wasn't included in payload from the front-end."),
        };

        let repo = self.borrow_repo()?;
        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        repo.revert(&commit, None)?;

        if !self.has_conflicts()? {
            repo.cleanup_state()?;
        }

        let commit = repo.find_commit(Oid::from_str(sha)?)?;

        if is_committing && !self.has_conflicts()? && self.has_staged_changes()? {
            let committer = repo.signature()?;
            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD has no target, failed to commit after revert."),
            };

            let mut new_full_message = String::from("Revert \"") + GitManager::get_utf8_string(commit.summary(), "Commit Summary")? + "\"";
            let (message_without_summary, uses_crlf) = GitManager::get_message_without_summary(GitManager::get_utf8_string(commit.message(), "Commit Message")?);
            if uses_crlf {
                new_full_message += "\r\n\r\n";
            } else {
                new_full_message += "\n\n";
            }
            new_full_message += message_without_summary.as_str();

            self.git_commit(new_full_message, &commit.author(), &committer, vec![&head_commit])?;
        }

        Ok(())
    }

    pub fn git_abort(&self) -> Result<()> {
        let repo = self.borrow_repo()?;

        let head_commit = match repo.head()?.target() {
            Some(oid) => repo.find_commit(oid)?,
            None => bail!("HEAD doesn't have a target commit, cannot abort to HEAD"),
        };

        repo.reset(head_commit.as_object(), ResetType::Hard, None)?;

        repo.cleanup_state()?;

        Ok(())
    }

    pub fn git_continue_cherrypick(&self) -> Result<()> {
        if !self.has_conflicts()? {
            let repo = self.borrow_repo()?;

            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD doesn't have a target commit (which is where a cherrypick operation starts on), can't complete cherrypick."),
            };

            let mut cherrypick_head_file = repo.path().to_path_buf();
            cherrypick_head_file.push("CHERRY_PICK_HEAD");
            let cherrypick_head_string = fs::read_to_string(cherrypick_head_file)?;
            let sha = cherrypick_head_string.trim();

            let commit_from_op = repo.find_commit(Oid::from_str(sha)?)?;

            let committer = repo.signature()?;

            self.git_commit(String::from(GitManager::get_utf8_string(commit_from_op.message(), "Commit Message")?), &commit_from_op.author(), &committer, vec![&head_commit])?;

            repo.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_continue_revert(&self) -> Result<()> {
        if !self.has_conflicts()? {
            let repo = self.borrow_repo()?;

            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD doesn't have a target commit (which is where a revert operation starts on), can't complete revert."),
            };

            let mut revert_file = repo.path().to_path_buf();
            revert_file.push("REVERT_HEAD");
            let revert_head_string = fs::read_to_string(revert_file)?;
            let sha = revert_head_string.trim();

            let commit_from_op = repo.find_commit(Oid::from_str(sha)?)?;

            let committer = repo.signature()?;

            let mut new_full_message = String::from("Revert \"") + GitManager::get_utf8_string(commit_from_op.summary(), "Commit Summary")? + "\"";
            let (message_without_summary, uses_crlf) = GitManager::get_message_without_summary(GitManager::get_utf8_string(commit_from_op.message(), "Commit Message")?);
            if uses_crlf {
                new_full_message += "\r\n\r\n";
            } else {
                new_full_message += "\n\n";
            }
            new_full_message += message_without_summary.as_str();

            self.git_commit(new_full_message, &commit_from_op.author(), &committer, vec![&head_commit])?;

            repo.cleanup_state()?;
        }

        Ok(())
    }

    pub fn git_continue_merge(&self) -> Result<()> {
        if !self.has_conflicts()? {
            let repo = self.borrow_repo()?;

            let head_commit = match repo.head()?.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("HEAD doesn't have a target commit (which is where a merge operation starts on), can't complete merge."),
            };

            let mut merge_file = repo.path().to_path_buf();
            merge_file.push("MERGE_HEAD");
            let merge_head_string = fs::read_to_string(merge_file)?;
            let sha = merge_head_string.trim();

            let commit_from_op = repo.find_commit(Oid::from_str(sha)?)?;

            let mut short_sha = String::from(sha.clone());
            short_sha.truncate(5);

            let mut head_short_sha = head_commit.id().to_string();
            head_short_sha.truncate(5);
            let message = String::from("Merge commit ") + short_sha.as_str() + " into commit " + head_short_sha.as_str();

            let committer = repo.signature()?;
            self.git_commit(message, &committer, &committer, vec![&head_commit, &commit_from_op])?;

            repo.cleanup_state()?;
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

    pub fn git_add_remote(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let remote_name = match json_hm.get("remote_name") {
            Some(s) => s,
            None => bail!("remote_name not included in payload from the front-end"),
        };
        let remote_url = match json_hm.get("remote_url") {
            Some(s) => s,
            None => bail!("remote_url not included in payload from the front-end"),
        };

        repo.remote(remote_name.as_str(), remote_url.as_str())?;

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

    pub fn git_checkout_from_json(&self, json_str: &str) -> Result<()> {
        let ref_name_value: Value = serde_json::from_str(json_str)?;
        let ref_name: &str = GitManager::get_string_from_serde_string(ref_name_value.as_str())?;
        self.git_checkout(&self.borrow_repo()?.find_reference(ref_name)?)?;
        Ok(())
    }

    pub fn git_checkout_detached_head(&self, json_str: &str) -> Result<()> {
        let sha_value: Value = serde_json::from_str(json_str)?;
        let sha: &str = GitManager::get_string_from_serde_string(sha_value.as_str())?;
        let repo = self.borrow_repo()?;

        let oid = Oid::from_str(sha)?;
        let tree = repo.find_commit(oid)?.tree()?;

        repo.checkout_tree(tree.as_object(), None)?;
        repo.set_head_detached(oid)?;
        Ok(())
    }

    pub fn git_checkout_remote(&self, json_string: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_string)?;
        let remote_branch_shortname = match json_hm.get("branch_shorthand") {
            Some(n) => n,
            None => bail!("JSON Data is missing branch_shorthand attribute."),
        };
        let remote_branch_full_name = match json_hm.get("full_branch_name") {
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
            local_branch_shortname += remote_branch_name_parts[i];
            if i < remote_branch_name_parts.len() - 1 {
                local_branch_shortname += "/";
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

    pub fn git_stage_from_json(&self, json_str: &str) -> Result<()> {
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_str)?;

        self.git_stage(diff_delta.get_status(), diff_delta.get_path())?;

        Ok(())
    }

    pub fn git_unstage(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;
        let diff_delta: ParseableDiffDelta = serde_json::from_str(json_str)?;

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

        let mut tree = None;
        match repo.head() {
            Ok(head_ref) => {
                let commit = match head_ref.target() {
                    Some(oid) => Some(repo.find_commit(oid)?),
                    None => None,
                };
                tree = match commit {
                    Some(c) => Some(c.tree()?),
                    None => None,
                };
            },
            Err(e) => {
                if e.code() != ErrorCode::UnbornBranch {
                    return Err(e.into());
                }
            },
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
            Some(mut patch) => {
                patch.print(&mut |_diff_delta, _diff_hunk_opt, diff_line| {
                    if diff_line.origin_value() == DiffLineType::FileHeader {
                        if let Ok(s) = get_content_from_diff_line(&diff_line) {
                            // Include file header if filemode has changed or the file was renamed.
                            let is_filemode_change = !s.contains("+++") && (s.contains("old mode") || s.contains("new mode"));
                            let is_renamed_file = s.contains("rename") || s.contains("similarity");
                            if is_filemode_change || is_renamed_file {
                                file_lines.push(LineInfo::SomeSeparator(s));
                            }
                        }
                    } else if diff_line.origin_value() == DiffLineType::HunkHeader {
                        if let Ok(s) = get_content_from_diff_line(&diff_line) {
                            file_lines.push(LineInfo::SomeSeparator(s));
                        }
                    } else if let Ok(fli) = FileLineInfo::from_diff_line(diff_line, &file_type) {
                        file_lines.push(LineInfo::SomeFileLineInfo(fli));
                    }
                    true
                })?;
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

    fn git_commit(&self, full_message: String, author: &Signature, committer: &Signature, parent_commits: Vec<&Commit>) -> Result<()> {
        if !self.has_staged_changes()? {
            bail!("Attempted to commit with no staged changes! Maybe stage some changes first?");
        }

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
            full_message += "\n\n";
            full_message += message.as_str();
        }

        let mut parents = vec![];
        match repo.head() {
            Ok(head_ref) => {
                if let Some(oid) = head_ref.target() {
                    let commit = repo.find_commit(oid)?;
                    parents.push(commit);
                }
            },
            Err(e) => {
                if e.code() != ErrorCode::UnbornBranch {
                    return Err(e.into());
                }
            },
        };

        // This is a hack for getting a slice of borrowed commits later.
        let parent_refs: Vec<&Commit> = parents.iter().map(|c| {
            c
        }).collect();

        self.git_commit(full_message, &signature, &signature, parent_refs)?;
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

    pub fn git_delete_local_branch(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let branch_shorthand = match json_hm.get("branch_shorthand") {
            Some(s) => s,
            None => bail!("branch_shorthand not included in payload from front-end!"),
        };
        let delete_remote_branch = match json_hm.get("delete_remote_branch") {
            Some(s) => s == "true",
            None => bail!("delete_remote_branch not included in payload from front-end!"),
        };

        let mut branch = repo.find_branch(branch_shorthand, BranchType::Local)?;

        if delete_remote_branch {
            let remote_branch = branch.upstream()?;
            self.git_delete_remote_branch(remote_branch)?;
        }

        branch.delete()?;
        Ok(())
    }

    fn git_delete_remote_branch(&self, remote_branch: Branch) -> Result<()> {
        let repo = self.borrow_repo()?;

        let branch_shorthand = GitManager::get_utf8_string(remote_branch.get().shorthand(), "Branch Shorthand")?;
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
        push_options.remote_callbacks(GitManager::get_remote_callbacks());

        let first_slash_index = match branch_shorthand.find("/") {
            Some(i) => i,
            None => bail!("Remote Branch doesn't seem to have a remote in its name?"),
        };
        let mut remote = repo.find_remote(&branch_shorthand[0..first_slash_index])?;
        let refspec = String::from(":refs/heads/") + &branch_shorthand[(first_slash_index + 1)..];
        remote.push(&[refspec.as_str()], Some(&mut push_options))?;
        Ok(())
    }

    pub fn git_delete_remote_branch_from_json(&self, json_str: &str) -> Result<()> {
        let branch_shorthand_value: Value = serde_json::from_str(json_str)?;
        let branch_shorthand: &str = GitManager::get_string_from_serde_string(branch_shorthand_value.as_str())?;
        let repo = self.borrow_repo()?;
        let remote_branch = repo.find_branch(branch_shorthand, BranchType::Remote)?;
        self.git_delete_remote_branch(remote_branch)?;
        Ok(())
    }

    pub fn git_delete_tag(&self, json_str: &str) -> Result<()> {
        let tag_name_value: Value = serde_json::from_str(json_str)?;
        let tag_name: &str = GitManager::get_string_from_serde_string(tag_name_value.as_str())?;
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
            fetch_options.remote_callbacks(GitManager::get_remote_callbacks());
            remote.fetch(empty_refspecs, Some(&mut fetch_options), None)?;
        }
        Ok(())
    }

    pub fn git_fast_forward_branch(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let branch_shorthand_value: Value = serde_json::from_str(json_str)?;
        let branch_shorthand: &str = GitManager::get_string_from_serde_string(branch_shorthand_value.as_str())?;

        // Fetch first to make sure everything's up to date.
        self.git_fetch()?;

        let mut local_branch = repo.find_branch(branch_shorthand, BranchType::Local)?;

        let remote_branch = local_branch.upstream()?;
        let remote_ref = remote_branch.get();
        let remote_target = match remote_ref.target() {
            Some(oid) => oid,
            None => bail!("Remote branch is not targeting a commit, cannot pull."),
        };

        let head = repo.head()?;
        let local_ref = local_branch.get_mut();

        // If fast-forwarding the branch currently checked out, need to update the working
        // directory too.
        if *local_ref == head {
            let commit = match remote_ref.target() {
                Some(oid) => repo.find_commit(oid)?,
                None => bail!("Remote branch has no target commit."),
            };
            let tree = commit.tree()?;
            repo.checkout_tree(tree.as_object(), None)?;
        }

        local_ref.set_target(remote_target, "oxidized_git fast-forward: setting new target for local ref")?;

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
                None => bail!("Remote branch has no target commit."),
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
            bail!("It looks like a pull may be possible, but your MergePreference(s) are preventing it. If you have merge.ff or pull.ff set to 'only' or 'false', consider unsetting it by running 'git config --global --unset merge.ff' or 'git config --global --unset pull.ff'");
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
        push_options.remote_callbacks(GitManager::get_remote_callbacks());

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

    pub fn git_push_tag(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let mut tag_full_name = match json_hm.get("tagFullName") {
            Some(s) => s.clone(),
            None => bail!("tagFullName not included in payload from front-end."),
        };
        let is_force = match json_hm.get("isForcePush") {
            Some(s) => s == "true",
            None => bail!("isForcePush not included in payload from front-end."),
        };
        let remote_name = match json_hm.get("selectedRemote") {
            Some(s) => s.as_str(),
            None => bail!("selectedRemote not included in payload from front-end."),
        };

        let mut remote = repo.find_remote(remote_name)?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(GitManager::get_remote_callbacks());

        if is_force {
            tag_full_name.insert(0, '+');
        }

        remote.push(&[tag_full_name.as_str()], Some(&mut push_options))?;

        Ok(())
    }

    pub fn git_stash(&mut self, json_str: &str) -> Result<()> {
        let message_value: Value = serde_json::from_str(json_str)?;
        let message: &str = GitManager::get_string_from_serde_string(message_value.as_str())?;
        let repo = self.borrow_repo_mut()?;

        if message == "" {
            repo.stash_save2(&repo.signature()?, None, Some(StashFlags::INCLUDE_UNTRACKED))?;
        } else {
            repo.stash_save2(&repo.signature()?, Some(message), Some(StashFlags::INCLUDE_UNTRACKED))?;
        }

        Ok(())
    }

    pub fn git_apply_stash(&mut self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo_mut()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let index = match json_hm.get("index") {
            Some(s) => {
                s.parse::<usize>()?
            },
            None => bail!("index not included in payload from front-end."),
        };
        let delete_stash = match json_hm.get("delete_stash") {
            Some(s) => s == "true",
            None => bail!("delete_stash not included in payload from front-end."),
        };

        if delete_stash {
            repo.stash_pop(index, None)?;
        } else {
            repo.stash_apply(index, None)?;
        }

        Ok(())
    }

    pub fn git_delete_stash(&mut self, json_str: &str) -> Result<()> {
        let stash_index_str_value: Value = serde_json::from_str(json_str)?;
        let stash_index_str: &str = GitManager::get_string_from_serde_string(stash_index_str_value.as_str())?;
        let repo = self.borrow_repo_mut()?;

        let index = stash_index_str.parse::<usize>()?;

        repo.stash_drop(index)?;

        Ok(())
    }

    pub fn git_branch(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let branch_options: HashMap<String, String> = serde_json::from_str(json_str)?;
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

    pub fn git_tag(&self, json_str: &str) -> Result<()> {
        let repo = self.borrow_repo()?;

        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let commit_sha = match json_hm.get("tag_sha") {
            Some(s) => {
                if s != "" {
                    s.clone()
                } else {
                    match repo.head()?.target() {
                        Some(oid) => oid.to_string(),
                        None => bail!("HEAD has no target to create a tag on."),
                    }
                }
            },
            None => bail!("tag_sha not included in payload from front-end."),
        };
        let is_lightweight = match json_hm.get("is_lightweight") {
            Some(s) => s == "true",
            None => bail!("is_lightweight not included in payload from front-end."),
        };
        let name = match json_hm.get("name") {
            Some(s) => s,
            None => bail!("name not included in payload from front-end."),
        };
        let message = match json_hm.get("message") {
            Some(s) => s,
            None => bail!("message not included in payload from front-end."),
        };

        let git_object = repo.find_object(Oid::from_str(&*commit_sha)?, Some(ObjectType::Commit))?;

        if is_lightweight {
            repo.tag_lightweight(name, &git_object, false)?;
        } else {
            let sig = repo.signature()?;
            repo.tag(name, &git_object, &sig, message, false)?;
        }

        Ok(())
    }

    #[allow(unused_unsafe)]
    fn get_remote_callbacks() -> RemoteCallbacks<'static> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            let config = match config_manager::get_config() {
                Ok(c) => c,
                Err(e) => return Err(git2::Error::from_str(&*format!("Error during config file read: {}", e))),
            };
            let cred_type = match config.borrow_cred_type() {
                Some(s) => s.clone(),
                None => return Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials")),
            };
            return if cred_type == "HTTPS" {
                let username = match config.borrow_https_username() {
                    Some(u) => u.clone(),
                    None => return Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials")),
                };
                let pass;
                unsafe {
                    pass = match keytar::get_password("oxidized_git", "password") {
                        Ok(p) => p,
                        Err(_) => return Err(git2::Error::from_str("Error finding password in keychain!")),
                    };
                }
                if pass.success {
                    Cred::userpass_plaintext(username.as_str(), &*pass.password)
                } else {
                    Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials"))
                }
            } else if cred_type == "SSH" {
                let username = match username_from_url {
                    Some(s) => s.clone(),
                    None => return Err(git2::Error::from_str("No username in Remote URL, did you use an SSH URL for your remote?")),
                };
                let public_key_path = match config.borrow_public_key_path() {
                    Some(p) => p,
                    None => return Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials")),
                };
                let private_key_path = match config.borrow_private_key_path() {
                    Some(p) => p,
                    None => return Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials")),
                };
                let uses_passphrase = match config.borrow_uses_passphrase() {
                    Some(b) => b,
                    None => return Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials")),
                };

                if *uses_passphrase {
                    let pass;
                    unsafe {
                        pass = match keytar::get_password("oxidized_git", "passphrase") {
                            Ok(p) => p,
                            Err(_) => return Err(git2::Error::from_str("Error finding passphrase in keychain!")),
                        };
                    }
                    if pass.success {
                        Cred::ssh_key(username, Some(public_key_path), private_key_path, Some(&*pass.password))
                    } else {
                        Err(git2::Error::from_str("Credentials are required to perform that operation. Please set your credentials in the menu bar under Security > Set Credentials"))
                    }
                } else {
                    Cred::ssh_key(username, Some(public_key_path), private_key_path, None)
                }
            } else {
                Err(git2::Error::from_str("Credential Type unrecognized. Please set your credentials in the menu bar under Security > Set Credentials"))
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
    pub fn set_https_credentials(&self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let username = match json_hm.get("username") {
            Some(u) => u.clone(),
            None => bail!("No username supplied"),
        };
        let password = match json_hm.get("password") {
            Some(p) => p,
            None => bail!("No password supplied"),
        };

        let mut config = config_manager::get_config()?;
        config.set_cred_type(String::from("HTTPS"));
        config.set_https_username(username);
        config.save()?;

        unsafe {
            keytar::set_password("oxidized_git", "password", password)?;
        }

        Ok(())
    }

    #[allow(unused_unsafe)]
    pub fn set_ssh_credentials(&self, json_str: &str) -> Result<()> {
        let json_hm: HashMap<String, String> = serde_json::from_str(json_str)?;
        let public_key_path = match json_hm.get("public_key_path") {
            Some(s) => s.clone(),
            None => bail!("No public_key_path supplied from front-end."),
        };
        let private_key_path = match json_hm.get("private_key_path") {
            Some(s) => s.clone(),
            None => bail!("No private_key_path supplied from front-end."),
        };
        let passphrase = match json_hm.get("passphrase") {
            Some(s) => s.clone(),
            None => bail!("No passphrase supplied from front-end."),
        };

        let mut config = config_manager::get_config()?;
        config.set_cred_type(String::from("SSH"));
        config.set_public_key_path(public_key_path.into());
        config.set_private_key_path(private_key_path.into());

        if passphrase != "" {
            config.set_uses_passphrase(true);
            unsafe {
                keytar::set_password("oxidized_git", "passphrase", &*passphrase)?;
            }
        } else {
            config.set_uses_passphrase(false);
        }

        config.save()?;

        Ok(())
    }
}

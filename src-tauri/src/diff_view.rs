use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use git2::{Delta, DiffFormat, DiffOptions, Oid, Repository, Sort, Tree};

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct DiffCommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub author_name: String,
    pub timestamp: DateTime<Local>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffCompareMode {
    LatestCommit,
    LatestAuthor,
    CompareCommits { from_sha: String, to_sha: String },
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct DiffFileEntry {
    pub path: String,
    pub status: String,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct DiffRangeInfo {
    pub from_sha: Option<String>,
    pub from_label: String,
    pub to_sha: String,
    pub to_label: String,
    pub author_name: String,
    pub timestamp: DateTime<Local>,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct DiffViewData {
    pub range: DiffRangeInfo,
    pub files: Vec<DiffFileEntry>,
    pub selected_file: Option<String>,
    pub diff_text: String,
}

struct ResolvedRange {
    base_oid: Option<Oid>,
    target_oid: Oid,
    author_name: String,
    timestamp: DateTime<Local>,
}

pub fn list_commits(repo_path: &str, limit: usize) -> Result<Vec<DiffCommitSummary>> {
    let repo = open_repo(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    revwalk.push_head()?;

    let mut commits = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        commits.push(DiffCommitSummary {
            sha: oid.to_string(),
            short_sha: short_sha(oid),
            author_name: commit_author_name(&commit),
            timestamp: commit_timestamp(&commit),
        });
    }

    Ok(commits)
}

pub fn load_view(
    repo_path: &str,
    mode: DiffCompareMode,
    selected_file: Option<&str>,
) -> Result<DiffViewData> {
    let repo = open_repo(repo_path)?;
    let resolved = resolve_range(&repo, &mode)?;

    let target_commit = repo.find_commit(resolved.target_oid)?;
    let target_tree = target_commit.tree()?;
    let base_commit = match resolved.base_oid {
        Some(oid) => Some(repo.find_commit(oid)?),
        None => None,
    };
    let base_tree = match base_commit.as_ref() {
        Some(commit) => Some(commit.tree()?),
        None => None,
    };

    let files = collect_files(&repo, base_tree.as_ref(), &target_tree)?;
    let selected_file = resolve_selected_file(selected_file, &files);
    let diff_text = match selected_file.as_deref() {
        Some(path) => render_diff_text(&repo, base_tree.as_ref(), &target_tree, path)?,
        None => String::new(),
    };

    Ok(DiffViewData {
        range: DiffRangeInfo {
            from_sha: resolved.base_oid.map(|oid| oid.to_string()),
            from_label: resolved
                .base_oid
                .map(short_sha)
                .unwrap_or_else(|| "Start".to_string()),
            to_sha: resolved.target_oid.to_string(),
            to_label: short_sha(resolved.target_oid),
            author_name: resolved.author_name,
            timestamp: resolved.timestamp,
        },
        files,
        selected_file,
        diff_text,
    })
}

fn open_repo(repo_path: &str) -> Result<Repository> {
    Repository::open(repo_path).with_context(|| format!("Failed to open repository: {repo_path}"))
}

fn resolve_range(repo: &Repository, mode: &DiffCompareMode) -> Result<ResolvedRange> {
    match mode {
        DiffCompareMode::LatestCommit => resolve_latest_commit(repo),
        DiffCompareMode::LatestAuthor => resolve_latest_author(repo),
        DiffCompareMode::CompareCommits { from_sha, to_sha } => {
            resolve_manual_range(repo, from_sha, to_sha)
        }
    }
}

fn resolve_latest_commit(repo: &Repository) -> Result<ResolvedRange> {
    let target = repo.head()?.peel_to_commit()?;
    Ok(ResolvedRange {
        base_oid: target.parent_id(0).ok(),
        target_oid: target.id(),
        author_name: commit_author_name(&target),
        timestamp: commit_timestamp(&target),
    })
}

fn resolve_latest_author(repo: &Repository) -> Result<ResolvedRange> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    revwalk.push_head()?;

    let Some(first_oid) = revwalk.next() else {
        return Err(anyhow!("Repository has no commits"));
    };
    let first_oid = first_oid?;
    let first_commit = repo.find_commit(first_oid)?;
    let target_author = normalize_author_name(&commit_author_name(&first_commit));

    let mut oldest_matching = first_commit.clone();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if normalize_author_name(&commit_author_name(&commit)) != target_author {
            break;
        }
        oldest_matching = commit;
    }

    Ok(ResolvedRange {
        base_oid: oldest_matching.parent_id(0).ok(),
        target_oid: first_commit.id(),
        author_name: commit_author_name(&first_commit),
        timestamp: commit_timestamp(&first_commit),
    })
}

fn resolve_manual_range(repo: &Repository, from_sha: &str, to_sha: &str) -> Result<ResolvedRange> {
    let mut from_oid = Oid::from_str(from_sha)?;
    let mut to_oid = Oid::from_str(to_sha)?;

    if from_oid != to_oid {
        let to_descends_from_from = repo.graph_descendant_of(to_oid, from_oid).unwrap_or(false);
        let from_descends_from_to = repo.graph_descendant_of(from_oid, to_oid).unwrap_or(false);

        if from_descends_from_to && !to_descends_from_from {
            std::mem::swap(&mut from_oid, &mut to_oid);
        } else if !to_descends_from_from && !from_descends_from_to {
            let from_commit = repo.find_commit(from_oid)?;
            let to_commit = repo.find_commit(to_oid)?;
            if from_commit.time().seconds() > to_commit.time().seconds() {
                std::mem::swap(&mut from_oid, &mut to_oid);
            }
        }
    }

    let target = repo.find_commit(to_oid)?;
    Ok(ResolvedRange {
        base_oid: Some(from_oid),
        target_oid: to_oid,
        author_name: commit_author_name(&target),
        timestamp: commit_timestamp(&target),
    })
}

fn collect_files(repo: &Repository, base_tree: Option<&Tree<'_>>, target_tree: &Tree<'_>) -> Result<Vec<DiffFileEntry>> {
    let mut diff_opts = DiffOptions::new();
    let mut diff = repo.diff_tree_to_tree(base_tree, Some(target_tree), Some(&mut diff_opts))?;
    diff.find_similar(None)?;

    let mut files = diff
        .deltas()
        .filter_map(|delta| delta_to_file_entry(repo, base_tree, Some(target_tree), &delta))
        .collect::<Vec<_>>();

    files.sort_by(|a, b| {
        let a_md = is_markdown_path(&a.path);
        let b_md = is_markdown_path(&b.path);
        b_md.cmp(&a_md).then_with(|| a.path.cmp(&b.path))
    });

    Ok(files)
}

fn delta_to_file_entry(
    repo: &Repository,
    base_tree: Option<&Tree<'_>>,
    target_tree: Option<&Tree<'_>>,
    delta: &git2::DiffDelta<'_>,
) -> Option<DiffFileEntry> {
    let path = delta_path(delta)?;
    if !delta_is_text(repo, base_tree, target_tree, delta) {
        return None;
    }

    Some(DiffFileEntry {
        path,
        status: delta_status(delta.status()).to_string(),
    })
}

fn resolve_selected_file(selected_file: Option<&str>, files: &[DiffFileEntry]) -> Option<String> {
    selected_file
        .filter(|path| files.iter().any(|file| file.path == *path))
        .map(ToOwned::to_owned)
        .or_else(|| files.first().map(|file| file.path.clone()))
}

fn render_diff_text(
    repo: &Repository,
    base_tree: Option<&Tree<'_>>,
    target_tree: &Tree<'_>,
    path: &str,
) -> Result<String> {
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(path);
    let mut diff = repo.diff_tree_to_tree(base_tree, Some(target_tree), Some(&mut diff_opts))?;
    diff.find_similar(None)?;

    let mut rendered = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        rendered.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    Ok(rendered)
}

fn delta_path(delta: &git2::DiffDelta<'_>) -> Option<String> {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(|path| path.to_string_lossy().to_string())
}

fn delta_status(status: Delta) -> &'static str {
    match status {
        Delta::Added => "added",
        Delta::Deleted => "deleted",
        Delta::Renamed => "renamed",
        Delta::Copied => "copied",
        Delta::Typechange => "typechange",
        _ => "modified",
    }
}

fn delta_is_text(
    repo: &Repository,
    base_tree: Option<&Tree<'_>>,
    target_tree: Option<&Tree<'_>>,
    delta: &git2::DiffDelta<'_>,
) -> bool {
    let old_path = delta.old_file().path();
    let new_path = delta.new_file().path();

    blob_is_text(repo, base_tree, old_path) || blob_is_text(repo, target_tree, new_path)
}

fn blob_is_text(repo: &Repository, tree: Option<&Tree<'_>>, path: Option<&Path>) -> bool {
    let Some(tree) = tree else {
        return false;
    };
    let Some(path) = path else {
        return false;
    };

    let Ok(entry) = tree.get_path(path) else {
        return false;
    };
    let Ok(obj) = entry.to_object(repo) else {
        return false;
    };
    let Ok(blob) = obj.peel_to_blob() else {
        return false;
    };

    !blob.is_binary()
}

fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".markdown")
}

fn commit_author_name(commit: &git2::Commit<'_>) -> String {
    commit.author().name().unwrap_or("Unknown").to_string()
}

fn normalize_author_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn commit_timestamp(commit: &git2::Commit<'_>) -> DateTime<Local> {
    local_timestamp(commit.time().seconds())
}

fn local_timestamp(seconds: i64) -> DateTime<Local> {
    Local
        .timestamp_opt(seconds, 0)
        .single()
        .unwrap_or_else(Local::now)
}

fn short_sha(oid: Oid) -> String {
    oid.to_string().chars().take(8).collect()
}

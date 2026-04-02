use git_sync_lib::{
    CommandGitTransport, RepositorySynchronizer, ResolvedFileContent, SyncConfig,
    FALLBACK_BRANCH_PREFIX,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

const BASE_CONTENT: &str = "base content\n";
const OURS_CONTENT: &str = "our changes\n";
const THEIRS_CONTENT: &str = "their changes\n";
const FALLBACK_BRANCH: &str = "git-sync/test-host-1234";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn git: {}", e));
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn setup_git_user(dir: &Path) {
    run_git(dir, &["config", "user.name", "Test User"]);
    run_git(dir, &["config", "user.email", "test@example.com"]);
}

fn read_file(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap()
}

fn head_is_clean(dir: &Path) -> bool {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

fn current_branch(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn make_syncer(
    repo_path: &Path,
    branch_name: &str,
    target_branch: &str,
) -> RepositorySynchronizer {
    RepositorySynchronizer::new_with_transport(
        repo_path,
        SyncConfig {
            branch_name: branch_name.to_string(),
            target_branch: Some(target_branch.to_string()),
            remote_name: "origin".to_string(),
            ..SyncConfig::default()
        },
        Arc::new(CommandGitTransport),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Creates a local + bare repo pair where `git merge origin/main` has been
/// attempted and left the index in conflict state (repo state == Merge).
///
/// Layout:
///   bare/   ← A (base), C (theirs) — origin
///   local1/ ← A, C (helper used to push C)
///   local/  ← A, B (ours) — git fetch done, merge attempted, CONFLICT
///
/// Returns the path to `local/`.
fn setup_direct_conflict(tmp: &TempDir) -> std::path::PathBuf {
    let bare = tmp.path().join("bare");
    let local1 = tmp.path().join("local1");
    let local = tmp.path().join("local");

    // ── bare repo ──────────────────────────────────────────────────────────
    run_git(tmp.path(), &["init", "--bare", "bare"]);
    // Ensure the default branch is "main" regardless of system git config.
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    // ── local1: initial commit A + "their" commit C ────────────────────────
    run_git(tmp.path(), &["clone", "bare", "local1"]);
    setup_git_user(&local1);
    // Ensure we're on "main" even if system default is "master".
    run_git(&local1, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    fs::write(local1.join("file.txt"), BASE_CONTENT).unwrap();
    run_git(&local1, &["add", "file.txt"]);
    run_git(&local1, &["commit", "-m", "initial"]);
    run_git(&local1, &["push", "origin", "HEAD:main"]);

    // ── local: clone (gets A), make "our" commit B (don't push) ───────────
    run_git(tmp.path(), &["clone", "bare", "local"]);
    setup_git_user(&local);

    fs::write(local.join("file.txt"), OURS_CONTENT).unwrap();
    run_git(&local, &["commit", "-am", "our changes"]);

    // ── local1: push "their" commit C ──────────────────────────────────────
    fs::write(local1.join("file.txt"), THEIRS_CONTENT).unwrap();
    run_git(&local1, &["commit", "-am", "their changes"]);
    run_git(&local1, &["push", "origin", "main"]);

    // ── local: fetch + attempt merge (expected to conflict — exit 1 is OK) ─
    run_git(&local, &["fetch", "origin"]);
    let _ = Command::new("git")
        .args(["merge", "--no-edit", "origin/main"])
        .current_dir(&local)
        .output()
        .unwrap();

    local
}

/// Creates a repo in the fallback-branch state that git-sync produces when
/// `conflict_branch = true`:
///
///   HEAD → refs/heads/git-sync/test-host-1234 → B (ours, parent=A)
///   refs/heads/main                            → C (theirs, parent=A)
///   refs/remotes/origin/main                   → C
///
/// Returns the path to `local/`.
fn setup_fallback_branch(tmp: &TempDir) -> std::path::PathBuf {
    let bare = tmp.path().join("bare");
    let helper = tmp.path().join("helper");
    let local = tmp.path().join("local");

    // Verify that FALLBACK_BRANCH starts with the expected prefix.
    assert!(FALLBACK_BRANCH.starts_with(FALLBACK_BRANCH_PREFIX));

    // ── bare + helper: commit A ─────────────────────────────────────────────
    run_git(tmp.path(), &["init", "--bare", "bare"]);
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    run_git(tmp.path(), &["clone", "bare", "helper"]);
    setup_git_user(&helper);
    run_git(&helper, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    fs::write(helper.join("file.txt"), BASE_CONTENT).unwrap();
    run_git(&helper, &["add", "file.txt"]);
    run_git(&helper, &["commit", "-m", "initial"]);
    run_git(&helper, &["push", "origin", "HEAD:main"]);

    // ── local: clone (gets A on main), create fallback branch + commit B ──
    run_git(tmp.path(), &["clone", "bare", "local"]);
    setup_git_user(&local);

    run_git(&local, &["checkout", "-b", FALLBACK_BRANCH]);
    fs::write(local.join("file.txt"), OURS_CONTENT).unwrap();
    run_git(&local, &["commit", "-am", "our changes"]);

    // ── helper: push "their" commit C to main ──────────────────────────────
    fs::write(helper.join("file.txt"), THEIRS_CONTENT).unwrap();
    run_git(&helper, &["commit", "-am", "their changes"]);
    run_git(&helper, &["push", "origin", "main"]);

    // ── local: fetch (refs/remotes/origin/main → C) + update local main ───
    run_git(&local, &["fetch", "origin"]);
    run_git(&local, &["branch", "-f", "main", "origin/main"]);

    local
}

// ---------------------------------------------------------------------------
// Group 1 — get_conflict_files_content
// ---------------------------------------------------------------------------

#[test]
fn direct_conflict_extracts_ours_theirs_base() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    let s = make_syncer(&local, "main", "main");

    let files = s.get_conflict_files_content().unwrap();
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.path, "file.txt");
    assert_eq!(f.ours, OURS_CONTENT);
    assert_eq!(f.theirs, THEIRS_CONTENT);
    assert_eq!(f.base, BASE_CONTENT);
}

#[test]
fn direct_conflict_extracts_multiple_files() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    let local1 = tmp.path().join("local1");

    // Abort the existing merge so we can rebuild with two conflicting files.
    run_git(&local, &["merge", "--abort"]);

    // local: add second.txt and commit (still on main, now 1 commit ahead of C)
    fs::write(local.join("second.txt"), "ours second\n").unwrap();
    run_git(&local, &["add", "second.txt"]);
    run_git(&local, &["commit", "-m", "add second.txt ours"]);

    // local1: add second.txt with different content, push
    fs::write(local1.join("second.txt"), "theirs second\n").unwrap();
    run_git(&local1, &["add", "second.txt"]);
    run_git(&local1, &["commit", "-m", "add second.txt theirs"]);
    run_git(&local1, &["push", "origin", "main"]);

    run_git(&local, &["fetch", "origin"]);
    let _ = Command::new("git")
        .args(["merge", "--no-edit", "origin/main"])
        .current_dir(&local)
        .output()
        .unwrap();

    let s = make_syncer(&local, "main", "main");
    let files = s.get_conflict_files_content().unwrap();
    assert_eq!(files.len(), 2, "expected 2 conflicting files, got: {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>());
}

#[test]
fn fallback_branch_extracts_via_in_memory_merge() {
    let tmp = TempDir::new().unwrap();
    let local = setup_fallback_branch(&tmp);
    let s = make_syncer(&local, FALLBACK_BRANCH, "main");

    let files = s.get_conflict_files_content().unwrap();
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.path, "file.txt");
    assert_eq!(f.ours, OURS_CONTENT);
    assert_eq!(f.theirs, THEIRS_CONTENT);
    assert_eq!(f.base, BASE_CONTENT);
}

// ---------------------------------------------------------------------------
// Group 2 — Direct conflict resolution
// ---------------------------------------------------------------------------

#[test]
fn direct_resolve_keep_mine() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    // `do_merge_with_favor` calls `repo.merge()` fresh; it needs the working
    // tree to be clean (no existing conflict markers), so abort first.
    run_git(&local, &["merge", "--abort"]);
    let s = make_syncer(&local, "main", "main");

    s.resolve_keep_mine().unwrap();

    assert_eq!(read_file(&local, "file.txt"), OURS_CONTENT);
    assert!(head_is_clean(&local), "working tree should be clean after resolve");
}

#[test]
fn direct_resolve_accept_remote() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    run_git(&local, &["merge", "--abort"]);
    let s = make_syncer(&local, "main", "main");

    s.resolve_accept_remote().unwrap();

    assert_eq!(read_file(&local, "file.txt"), THEIRS_CONTENT);
    assert!(head_is_clean(&local), "working tree should be clean after resolve");
}

#[test]
fn direct_complete_merge_with_ours_content() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    let s = make_syncer(&local, "main", "main");

    s.complete_conflict_merge(vec![ResolvedFileContent {
        path: "file.txt".to_string(),
        content: OURS_CONTENT.to_string(),
    }])
    .unwrap();

    assert_eq!(read_file(&local, "file.txt"), OURS_CONTENT);
    assert!(head_is_clean(&local), "working tree should be clean after merge");
}

#[test]
fn direct_complete_merge_with_custom_content() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    let s = make_syncer(&local, "main", "main");

    s.complete_conflict_merge(vec![ResolvedFileContent {
        path: "file.txt".to_string(),
        content: "custom\n".to_string(),
    }])
    .unwrap();

    assert_eq!(read_file(&local, "file.txt"), "custom\n");
    assert!(head_is_clean(&local), "working tree should be clean after merge");
}

// ---------------------------------------------------------------------------
// Group 3 — Fallback branch resolution
// ---------------------------------------------------------------------------

#[test]
fn fallback_complete_merge_lands_on_target() {
    let tmp = TempDir::new().unwrap();
    let local = setup_fallback_branch(&tmp);
    let s = make_syncer(&local, FALLBACK_BRANCH, "main");

    s.complete_conflict_merge(vec![ResolvedFileContent {
        path: "file.txt".to_string(),
        content: "merged\n".to_string(),
    }])
    .unwrap();

    assert_eq!(current_branch(&local), "main");
    assert_eq!(read_file(&local, "file.txt"), "merged\n");
    assert!(head_is_clean(&local), "working tree should be clean after merge");
}

#[test]
fn fallback_complete_merge_creates_merge_commit() {
    let tmp = TempDir::new().unwrap();
    let local = setup_fallback_branch(&tmp);
    let s = make_syncer(&local, FALLBACK_BRANCH, "main");

    s.complete_conflict_merge(vec![ResolvedFileContent {
        path: "file.txt".to_string(),
        content: "merged\n".to_string(),
    }])
    .unwrap();

    // %P prints space-separated parent OIDs; a merge commit has 2.
    let out = Command::new("git")
        .args(["log", "--pretty=format:%P", "-1"])
        .current_dir(&local)
        .output()
        .unwrap();
    let parents = String::from_utf8_lossy(&out.stdout);
    let parent_count = parents.split_whitespace().count();
    assert_eq!(
        parent_count, 2,
        "HEAD should be a merge commit with 2 parents, got parents: {:?}",
        parents.trim()
    );
}

#[test]
fn fallback_abandon_returns_to_target() {
    let tmp = TempDir::new().unwrap();
    let local = setup_fallback_branch(&tmp);
    let s = make_syncer(&local, FALLBACK_BRANCH, "main");

    s.abandon_conflict_branch().unwrap();

    assert_eq!(current_branch(&local), "main");

    let out = Command::new("git")
        .args(["branch", "--list", FALLBACK_BRANCH])
        .current_dir(&local)
        .output()
        .unwrap();
    let branch_list = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(
        branch_list.is_empty(),
        "fallback branch should have been deleted, but found: {:?}",
        branch_list
    );
}

// ---------------------------------------------------------------------------
// Group 4 — get_conflict_info
// ---------------------------------------------------------------------------

#[test]
fn conflict_info_direct_lists_conflicted_files() {
    let tmp = TempDir::new().unwrap();
    let local = setup_direct_conflict(&tmp);
    let s = make_syncer(&local, "main", "main");

    let paths = s.get_conflict_info().unwrap();
    assert!(
        paths.contains(&"file.txt".to_string()),
        "expected file.txt in conflict info, got: {:?}",
        paths
    );
}

#[test]
fn conflict_info_fallback_lists_would_conflict() {
    let tmp = TempDir::new().unwrap();
    let local = setup_fallback_branch(&tmp);
    let s = make_syncer(&local, FALLBACK_BRANCH, "main");

    let paths = s.get_conflict_info().unwrap();
    assert!(
        paths.contains(&"file.txt".to_string()),
        "expected file.txt in conflict info, got: {:?}",
        paths
    );
}

//! End-to-end worktree tests against real `git` repos built in tempdirs.

use std::path::Path;
use std::process::Command;

use crate::{
    base_repo_root, create_worktree, list_worktrees, merge_worktree, remove_worktree, Error,
};

/// A throwaway repo on a deterministic `main` branch, `.codeforge-worktrees/`
/// already gitignored so the base stays clean once worktrees are spun off.
/// Shared with `branches_tests`.
pub(crate) struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn init() -> Self {
        let f = Fixture { dir: tempfile::tempdir().expect("tempdir") };
        git_in(f.path(), &["init", "-q", "-b", "main"]);
        git_in(f.path(), &["config", "user.email", "t@t.t"]);
        git_in(f.path(), &["config", "user.name", "test"]);
        git_in(f.path(), &["config", "commit.gpgsign", "false"]);
        write_in(f.path(), ".gitignore", ".codeforge-worktrees/\n");
        write_in(f.path(), "README.md", "base\n");
        commit_in(f.path(), "init");
        f
    }

    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, rel: &str, contents: &str) {
        write_in(self.path(), rel, contents);
    }
}

pub(crate) fn git_in(dir: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(dir).output().expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_in(dir: &Path, rel: &str, contents: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(p, contents).expect("write fixture");
}

fn commit_in(dir: &Path, msg: &str) {
    git_in(dir, &["add", "-A"]);
    git_in(dir, &["commit", "-qm", msg]);
}

fn raw_status(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .expect("spawn git");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test]
async fn create_lists_new_worktree_with_branch_and_ahead_behind() {
    let f = Fixture::init();
    let wt = create_worktree(f.path(), "My Feature", None).await.unwrap();
    assert_eq!(wt.branch.as_deref(), Some("my-feature"));
    assert!(!wt.is_base);
    assert_eq!((wt.ahead, wt.behind), (0, 0));
    assert!(!wt.dirty);

    let list = list_worktrees(f.path()).await.unwrap();
    assert_eq!(list.len(), 2);
    let base = list.iter().find(|w| w.is_base).unwrap();
    assert_eq!(base.branch.as_deref(), Some("main"));
    assert_eq!((base.ahead, base.behind), (0, 0));
    let feat = list.iter().find(|w| !w.is_base).unwrap();
    assert_eq!(feat.branch.as_deref(), Some("my-feature"));
    assert_eq!((feat.ahead, feat.behind), (0, 0));

    // A commit on the worktree branch → 1 ahead; a commit on base → also 1 behind.
    write_in(&wt.path, "feat.txt", "x\n");
    commit_in(&wt.path, "feat work");
    f.write("base2.txt", "y\n");
    commit_in(f.path(), "base work");

    let list = list_worktrees(f.path()).await.unwrap();
    let feat = list.iter().find(|w| !w.is_base).unwrap();
    assert_eq!((feat.ahead, feat.behind), (1, 1));
}

#[tokio::test]
async fn dirty_flag_tracks_uncommitted_changes() {
    let f = Fixture::init();
    let wt = create_worktree(f.path(), "dirtywork", None).await.unwrap();

    let list = list_worktrees(f.path()).await.unwrap();
    assert!(list.iter().all(|w| !w.dirty), "fresh worktree + gitignored base are clean");

    write_in(&wt.path, "scratch.txt", "wip\n");
    let list = list_worktrees(f.path()).await.unwrap();
    assert!(list.iter().find(|w| !w.is_base).unwrap().dirty);
    assert!(!list.iter().find(|w| w.is_base).unwrap().dirty);
}

#[tokio::test]
async fn clean_merge_reports_merged() {
    let f = Fixture::init();
    let wt = create_worktree(f.path(), "cleanmerge", None).await.unwrap();
    write_in(&wt.path, "feature.txt", "hello\n");
    commit_in(&wt.path, "add feature");

    let res = merge_worktree(f.path(), &wt.path).await.unwrap();
    assert!(res.merged);
    assert!(res.conflicts.is_empty());
    assert!(!res.aborted);
    assert_eq!(res.source_branch, "cleanmerge");
    assert_eq!(res.target_branch, "main");
    assert!(f.path().join("feature.txt").exists(), "merged file landed in base");
}

#[tokio::test]
async fn conflicting_merge_reports_conflicts_and_aborts() {
    let f = Fixture::init();
    f.write("shared.txt", "base\n");
    commit_in(f.path(), "add shared");

    let wt = create_worktree(f.path(), "conflict", None).await.unwrap();
    write_in(&wt.path, "shared.txt", "feature-side\n");
    commit_in(&wt.path, "feature edit");
    f.write("shared.txt", "mainline-side\n");
    commit_in(f.path(), "main edit");

    let res = merge_worktree(f.path(), &wt.path).await.unwrap();
    assert!(!res.merged);
    assert!(res.aborted);
    assert!(res.conflicts.iter().any(|c| c == "shared.txt"), "conflicts: {:?}", res.conflicts);

    // Aborted merge leaves the base tree clean — no MERGE_HEAD, base's content restored.
    assert!(raw_status(f.path()).trim().is_empty(), "base not clean after abort");
    assert!(!f.path().join(".git").join("MERGE_HEAD").exists());
    let content = std::fs::read_to_string(f.path().join("shared.txt")).unwrap();
    assert_eq!(content, "mainline-side\n");
}

#[tokio::test]
async fn remove_worktree_drops_it_from_list() {
    let f = Fixture::init();
    let wt = create_worktree(f.path(), "removeme", None).await.unwrap();
    assert_eq!(list_worktrees(f.path()).await.unwrap().len(), 2);

    remove_worktree(f.path(), &wt.path, false).await.unwrap();
    let list = list_worktrees(f.path()).await.unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].is_base);
    assert!(!wt.path.exists());
}

#[tokio::test]
async fn refuse_remove_base_worktree() {
    let f = Fixture::init();
    let base = base_repo_root(f.path()).await.unwrap();
    let err = remove_worktree(f.path(), &base, false).await.unwrap_err();
    assert!(matches!(err, Error::CannotRemoveBase(_)), "got {err:?}");
}

#[tokio::test]
async fn refuse_recreate_existing_path_then_branch() {
    let f = Fixture::init();
    let wt = create_worktree(f.path(), "dup", None).await.unwrap();

    // Path still present → path collision, not a silent reuse.
    let err = create_worktree(f.path(), "dup", None).await.unwrap_err();
    assert!(matches!(err, Error::WorktreePathExists(_)), "got {err:?}");

    // Remove the worktree dir but keep the branch → branch collision.
    remove_worktree(f.path(), &wt.path, false).await.unwrap();
    let err = create_worktree(f.path(), "dup", None).await.unwrap_err();
    assert!(matches!(err, Error::BranchExists(_)), "got {err:?}");
}

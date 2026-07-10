//! End-to-end branch tests against real `git` repos (fixtures shared with
//! `worktree_tests`): list_branches, fetch_remotes, worktree_for_branch.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::worktree_tests::{git_in, Fixture};
use crate::{fetch_remotes, list_branches, worktree_for_branch, Error};

/// A bare "remote" repo wired up as `origin` of the fixture. Keep the returned
/// tempdir alive for the duration of the test.
fn add_origin(f: &Fixture) -> tempfile::TempDir {
    let remote = tempfile::tempdir().expect("tempdir");
    git_in(remote.path(), &["init", "-q", "--bare", "-b", "main"]);
    let url = remote.path().to_str().expect("utf8 tempdir path");
    git_in(f.path(), &["remote", "add", "origin", url]);
    remote
}

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").args(args).current_dir(dir).output().expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).expect("canonicalize")
}

#[tokio::test]
async fn list_branches_reports_locals_then_remotes_with_checkout_state() {
    let f = Fixture::init();
    git_in(f.path(), &["branch", "zeta"]);
    git_in(f.path(), &["branch", "alpha"]);
    let _remote = add_origin(&f);
    git_in(f.path(), &["push", "-q", "origin", "main", "alpha"]);
    fetch_remotes(f.path()).await.unwrap();
    // origin/HEAD is a symref, not a branch — it must be skipped, not listed.
    git_in(f.path(), &["remote", "set-head", "origin", "main"]);

    // Check `zeta` out into a worktree so its checked_out_at is filled.
    let wt = worktree_for_branch(f.path(), "zeta", None).await.unwrap();

    let branches = list_branches(f.path()).await.unwrap();
    let names: Vec<(&str, Option<&str>)> =
        branches.iter().map(|b| (b.name.as_str(), b.remote.as_deref())).collect();
    assert_eq!(
        names,
        vec![
            ("alpha", None),
            ("main", None),
            ("zeta", None),
            ("alpha", Some("origin")),
            ("main", Some("origin")),
        ],
        "locals first, then remotes, each alphabetical; no origin/HEAD"
    );

    let main = &branches[1];
    assert!(main.is_head, "main is the base HEAD");
    assert_eq!(main.checked_out_at.as_deref().map(canon), Some(canon(f.path())));

    let alpha = &branches[0];
    assert!(!alpha.is_head);
    assert_eq!(alpha.checked_out_at, None);

    let zeta = &branches[2];
    assert!(!zeta.is_head);
    assert_eq!(zeta.checked_out_at.as_deref().map(canon), Some(canon(&wt.path)));

    let remote_main = &branches[4];
    assert!(!remote_main.is_head);
    assert_eq!(remote_main.checked_out_at, None);
}

#[tokio::test]
async fn worktree_for_local_branch_checks_out_without_new_branch() {
    let f = Fixture::init();
    git_in(f.path(), &["branch", "feat/login"]);

    let wt = worktree_for_branch(f.path(), "feat/login", None).await.unwrap();
    assert_eq!(wt.branch.as_deref(), Some("feat/login"));
    assert!(!wt.is_base);
    assert!(
        wt.path.ends_with(".codeforge-worktrees/feat-login"),
        "slugged path, got {:?}",
        wt.path
    );
    // Checked the EXISTING branch out — no extra branch materialized.
    let heads = git_stdout(f.path(), &["for-each-ref", "refs/heads", "--format=%(refname:short)"]);
    assert_eq!(heads.lines().count(), 2, "main + feat/login only, got {heads:?}");
}

#[tokio::test]
async fn worktree_for_checked_out_branch_is_a_named_error_with_path() {
    let f = Fixture::init();
    // `main` is checked out at the base itself.
    let err = worktree_for_branch(f.path(), "main", None).await.unwrap_err();
    match err {
        Error::BranchCheckedOut { branch, path } => {
            assert_eq!(branch, "main");
            assert_eq!(canon(&path), canon(f.path()));
        }
        other => panic!("expected BranchCheckedOut, got {other:?}"),
    }
}

#[tokio::test]
async fn worktree_for_remote_branch_creates_tracking_branch() {
    let f = Fixture::init();
    let _remote = add_origin(&f);
    git_in(f.path(), &["branch", "feat/api"]);
    git_in(f.path(), &["push", "-q", "origin", "main", "feat/api"]);
    git_in(f.path(), &["branch", "-D", "feat/api"]);
    fetch_remotes(f.path()).await.unwrap();

    let wt = worktree_for_branch(f.path(), "feat/api", Some("origin")).await.unwrap();
    assert_eq!(wt.branch.as_deref(), Some("feat/api"));
    let upstream = git_stdout(&wt.path, &["rev-parse", "--abbrev-ref", "feat/api@{upstream}"]);
    assert_eq!(upstream, "origin/feat/api", "local branch tracks the remote one");
}

#[tokio::test]
async fn remote_mode_refuses_when_local_branch_exists() {
    let f = Fixture::init();
    let _remote = add_origin(&f);
    git_in(f.path(), &["push", "-q", "origin", "main"]);
    fetch_remotes(f.path()).await.unwrap();

    // A local `main` exists → picking origin/main is the named BranchExists.
    let err = worktree_for_branch(f.path(), "main", Some("origin")).await.unwrap_err();
    assert!(matches!(err, Error::BranchExists(_)), "got {err:?}");
    assert!(err.to_string().contains("pick the local branch"), "got {err}");
}

#[tokio::test]
async fn fetch_remotes_ok_without_remotes_and_errors_on_broken_remote() {
    let f = Fixture::init();
    // No remotes: a real, fine state — Ok, not an error.
    fetch_remotes(f.path()).await.unwrap();

    // A remote that doesn't resolve surfaces git's stderr verbatim.
    git_in(f.path(), &["remote", "add", "origin", "/nonexistent/definitely-not-a-repo"]);
    let err = fetch_remotes(f.path()).await.unwrap_err();
    assert!(matches!(err, Error::Git(_)), "got {err:?}");
}

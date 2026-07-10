//! End-to-end tests against real `git` repos built in tempdirs.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use forge_core::{Feature, FeatureFile, FileRole};
use forge_index::FeatureIndex;

use crate::{changed_files, collect_file_diffs, current_branch, current_head, diff_by_feature, init_repo};

/// A throwaway git repo. `git()` panics on non-zero exit so a broken fixture
/// fails loudly rather than masquerading as a parser bug.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn init() -> Self {
        let f = Fixture { dir: tempfile::tempdir().expect("tempdir") };
        f.git(&["init", "-q"]);
        f.git(&["config", "user.email", "t@t.t"]);
        f.git(&["config", "user.name", "test"]);
        f.git(&["config", "commit.gpgsign", "false"]);
        f
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write(&self, rel: &str, contents: &str) {
        let p = self.path().join(rel);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).expect("mkdir");
        }
        std::fs::write(p, contents).expect("write fixture");
    }

    fn write_bytes(&self, rel: &str, contents: &[u8]) {
        std::fs::write(self.path().join(rel), contents).expect("write fixture");
    }

    fn commit_all(&self, msg: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-qm", msg]);
    }
}

fn feature(slug: &str, files: &[&str]) -> Feature {
    Feature {
        slug: slug.into(),
        name: format!("{slug} feature"),
        description: String::new(),
        entry_points: Vec::new(),
        files: files
            .iter()
            .map(|p| FeatureFile { path: PathBuf::from(p), role: FileRole::Core, pinned: false })
            .collect(),
        tags: Vec::new(),
        pinned: false,
        color: None,
        group: None,
        updated_at: Utc::now(),
    }
}

fn find<'a>(files: &'a [forge_core::FileDiff], path: &str) -> &'a forge_core::FileDiff {
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no diff for {path}"))
}

/// `(origin, old_no, new_no)` triples of one hunk — the sequence under test.
fn seq(hunk: &forge_core::DiffHunk) -> Vec<(char, Option<u32>, Option<u32>)> {
    hunk.lines.iter().map(|l| (l.origin, l.old_no, l.new_no)).collect()
}

#[tokio::test]
async fn changed_files_reports_modified_staged_untracked_renamed() {
    let f = Fixture::init();
    f.write("modified.rs", "a\n");
    f.write("to_rename.rs", "keep\n");
    f.commit_all("init");

    f.write("modified.rs", "a\nb\n"); // unstaged edit
    f.git(&["mv", "to_rename.rs", "renamed.rs"]); // staged rename
    f.write("staged.rs", "new\n");
    f.git(&["add", "staged.rs"]); // staged add
    f.write("untracked.rs", "loose\n"); // untracked

    let got: BTreeSet<PathBuf> = changed_files(f.path()).await.unwrap().into_iter().collect();
    let want: BTreeSet<PathBuf> = ["modified.rs", "renamed.rs", "staged.rs", "untracked.rs"]
        .iter()
        .map(PathBuf::from)
        .collect();
    assert_eq!(got, want);
}

#[tokio::test]
async fn init_repo_creates_repo_on_unborn_main() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    assert!(!root.join(".git").exists());

    init_repo(root).await.unwrap();

    assert!(root.join(".git").is_dir(), "git init created .git (is_git_repo would be true)");
    assert_eq!(current_branch(root).await.unwrap().as_deref(), Some("main"));
    // Deliberately no commit: unborn main is the intended state.
    assert_eq!(current_head(root).await.unwrap(), None);
}

#[tokio::test]
async fn current_head_none_when_unborn_some_after_commit() {
    let f = Fixture::init();
    assert_eq!(current_head(f.path()).await.unwrap(), None);

    f.write("a.txt", "x\n");
    f.commit_all("init");
    let head = current_head(f.path()).await.unwrap().expect("head after commit");
    assert_eq!(head.len(), 40);
    assert!(head.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn multi_hunk_line_numbering_is_exact() {
    let f = Fixture::init();
    let base: String = (1..=12).map(|i| format!("line{i}\n")).collect();
    f.write("f.txt", &base);
    f.commit_all("init");
    // Two far-apart single-line edits → two independent unified=3 hunks.
    let edited = base.replace("line2\n", "line2X\n").replace("line11\n", "line11X\n");
    f.write("f.txt", &edited);

    let files = collect_file_diffs(f.path()).await.unwrap();
    let fd = find(&files, "f.txt");
    assert_eq!(fd.status, "modified");
    assert_eq!((fd.additions, fd.deletions), (2, 2));
    assert_eq!(fd.hunks.len(), 2);

    assert_eq!(
        seq(&fd.hunks[0]),
        vec![
            (' ', Some(1), Some(1)),
            ('-', Some(2), None),
            ('+', None, Some(2)),
            (' ', Some(3), Some(3)),
            (' ', Some(4), Some(4)),
            (' ', Some(5), Some(5)),
        ]
    );
    assert_eq!(
        seq(&fd.hunks[1]),
        vec![
            (' ', Some(8), Some(8)),
            (' ', Some(9), Some(9)),
            (' ', Some(10), Some(10)),
            ('-', Some(11), None),
            ('+', None, Some(11)),
            (' ', Some(12), Some(12)),
        ]
    );
}

#[tokio::test]
async fn renamed_file_parses_with_no_hunks() {
    let f = Fixture::init();
    f.write("orig.txt", "alpha\nbeta\ngamma\n");
    f.commit_all("init");
    f.git(&["mv", "orig.txt", "renamed.txt"]);

    let files = collect_file_diffs(f.path()).await.unwrap();
    let fd = find(&files, "renamed.txt");
    assert_eq!(fd.status, "renamed");
    assert_eq!((fd.additions, fd.deletions), (0, 0));
    assert!(fd.hunks.is_empty());
}

#[tokio::test]
async fn unborn_head_diffs_staged_file_as_added() {
    let f = Fixture::init();
    f.write("staged.txt", "hello\nworld\n");
    f.git(&["add", "staged.txt"]);
    f.write("loose.txt", "loose\n"); // untracked, synthesized

    let files = collect_file_diffs(f.path()).await.unwrap();

    let staged = find(&files, "staged.txt");
    assert_eq!(staged.status, "added");
    assert_eq!((staged.additions, staged.deletions), (2, 0));
    assert_eq!(
        seq(&staged.hunks[0]),
        vec![('+', None, Some(1)), ('+', None, Some(2))]
    );

    let loose = find(&files, "loose.txt");
    assert_eq!(loose.status, "untracked");
    assert_eq!(loose.additions, 1);
}

#[tokio::test]
async fn binary_untracked_shown_as_binary_marker() {
    let f = Fixture::init();
    f.write("seed.txt", "x\n");
    f.commit_all("init");
    f.write_bytes("bin.dat", b"\x00\x01\x02payload");
    f.write("text.txt", "readable\n");

    let files = collect_file_diffs(f.path()).await.unwrap();
    assert!(files.iter().any(|d| d.path == "text.txt"));
    let bin = find(&files, "bin.dat");
    assert!(bin.binary, "binary untracked file appears with binary:true, not dropped");
    assert!(bin.hunks.is_empty());
    assert_eq!((bin.additions, bin.deletions), (0, 0));
}

#[tokio::test]
async fn oversized_diff_is_truncated_once_centrally() {
    let f = Fixture::init();
    f.write("seed.txt", "x\n");
    f.commit_all("init");
    // An untracked file well over the cap: additions carry the true total, but
    // rendered lines are capped and `truncated` is set exactly once.
    let body: String = (1..=2500).map(|i| format!("line {i}\n")).collect();
    f.write("big.txt", &body);

    let files = collect_file_diffs(f.path()).await.unwrap();
    let fd = find(&files, "big.txt");
    assert!(fd.truncated, "capped file carries the typed truncated flag");
    assert_eq!(fd.additions, 2500, "true total preserved on additions");
    let shown: usize = fd.hunks.iter().map(|h| h.lines.len()).sum();
    assert_eq!(shown, 2000, "rendered lines capped at the single authoritative limit");
}

#[tokio::test]
async fn diff_by_feature_shared_file_and_unmapped_bucket() {
    let f = Fixture::init();
    for name in ["auth.rs", "billing.rs", "shared.rs", "mystery.rs"] {
        f.write(name, "start\n");
    }
    f.commit_all("init");
    f.write("auth.rs", "start\nauth-change\n");
    f.write("billing.rs", "start\nbilling-change\n");
    f.write("shared.rs", "start\nshared-change\n");
    f.write("mystery.rs", "start\nmystery-change\n");

    let mut index = FeatureIndex::load(f.path()).unwrap();
    index.upsert(feature("auth", &["auth.rs", "shared.rs"]));
    index.upsert(feature("billing", &["billing.rs", "shared.rs"]));

    let diff = diff_by_feature(f.path(), &index).await.unwrap();

    // unmapped is always last.
    let last = diff.groups.last().unwrap();
    assert_eq!(last.slug, "unmapped");
    assert_eq!(last.name, "Unmapped");
    assert!(!last.shared);
    assert_eq!(last.files.len(), 1);
    assert_eq!(last.files[0].path, "mystery.rs");

    let non_unmapped: BTreeSet<&str> =
        diff.groups.iter().filter(|g| g.slug != "unmapped").map(|g| g.slug.as_str()).collect();
    assert_eq!(non_unmapped, BTreeSet::from(["auth", "billing"]));

    let auth = diff.groups.iter().find(|g| g.slug == "auth").unwrap();
    let billing = diff.groups.iter().find(|g| g.slug == "billing").unwrap();
    assert_eq!(auth.name, "auth feature");
    // shared.rs belongs to two features → each group flagged shared, file in both.
    assert!(auth.shared && billing.shared);
    assert!(auth.files.iter().any(|d| d.path == "shared.rs"));
    assert!(billing.files.iter().any(|d| d.path == "shared.rs"));
    assert!(auth.files.iter().any(|d| d.path == "auth.rs"));
    assert!(billing.files.iter().any(|d| d.path == "billing.rs"));
}

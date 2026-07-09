//! Synthesizes all-added [`FileDiff`]s for untracked files.

use std::path::Path;

use forge_core::{DiffHunk, DiffLine, FileDiff};

const SNIFF_BYTES: usize = 8192;

/// Read an untracked file and present it as a single all-added hunk.
/// `None` only when the file is unreadable. Binary files (NUL byte in the first
/// 8 KiB) come back as a zero-hunk `binary: true` marker — the same shape a
/// tracked binary gets — so the review can show "binary, not shown" instead of
/// silently omitting the file. Over-long diffs are capped once, centrally, in
/// `collect_file_diffs`.
pub(crate) fn synthesize(repo_root: &Path, rel: &Path) -> Option<FileDiff> {
    let abs = repo_root.join(rel);
    let path = rel.to_string_lossy().into_owned();
    let bytes = match std::fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(path = %abs.display(), error = %e, "skipping unreadable untracked file");
            return None;
        }
    };
    if bytes[..bytes.len().min(SNIFF_BYTES)].contains(&0) {
        return Some(FileDiff {
            path,
            status: "untracked".into(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            binary: true,
            truncated: false,
        });
    }

    let text = String::from_utf8_lossy(&bytes);
    let total = text.lines().count();
    let lines: Vec<DiffLine> = text
        .lines()
        .enumerate()
        .map(|(i, l)| DiffLine {
            origin: '+',
            content: l.to_string(),
            old_no: None,
            new_no: Some(i as u32 + 1),
        })
        .collect();
    let hunks = if lines.is_empty() {
        Vec::new()
    } else {
        vec![DiffHunk { header: format!("@@ -0,0 +1,{total} @@"), lines }]
    };
    Some(FileDiff {
        path,
        status: "untracked".into(),
        hunks,
        additions: total as u32,
        deletions: 0,
        binary: false,
        truncated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, bytes: &[u8]) {
        std::fs::write(dir.join(rel), bytes).unwrap();
    }

    #[test]
    fn text_file_becomes_all_added_hunk() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", b"one\ntwo\nthree\n");
        let fd = synthesize(dir.path(), Path::new("a.txt")).unwrap();
        assert_eq!(fd.status, "untracked");
        assert_eq!(fd.additions, 3);
        assert_eq!(fd.deletions, 0);
        assert_eq!(fd.hunks.len(), 1);
        assert_eq!(fd.hunks[0].header, "@@ -0,0 +1,3 @@");
        let nos: Vec<_> = fd.hunks[0].lines.iter().map(|l| (l.origin, l.old_no, l.new_no)).collect();
        assert_eq!(nos, vec![('+', None, Some(1)), ('+', None, Some(2)), ('+', None, Some(3))]);
    }

    #[test]
    fn binary_file_shown_as_binary_marker() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "bin.dat", b"\x00\x01\x02payload");
        let fd = synthesize(dir.path(), Path::new("bin.dat")).unwrap();
        assert_eq!(fd.status, "untracked");
        assert!(fd.binary);
        assert!(fd.hunks.is_empty());
        assert_eq!((fd.additions, fd.deletions), (0, 0));
        assert!(!fd.truncated);
    }

    #[test]
    fn long_file_synthesized_in_full_truncation_is_central() {
        // synthesize no longer caps; the single authoritative cap lives in
        // `truncate::cap_file_diffs` (applied by `collect_file_diffs`).
        let dir = tempfile::tempdir().unwrap();
        let body: String = (1..=2050).map(|i| format!("line {i}\n")).collect();
        write(dir.path(), "big.txt", body.as_bytes());
        let fd = synthesize(dir.path(), Path::new("big.txt")).unwrap();
        assert_eq!(fd.additions, 2050);
        assert!(!fd.truncated, "synthesize does not set truncated");
        assert_eq!(fd.hunks[0].lines.len(), 2050);
        assert_eq!(fd.hunks[0].lines[2049].new_no, Some(2050));
    }

    #[test]
    fn empty_file_has_no_hunks() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "empty.txt", b"");
        let fd = synthesize(dir.path(), Path::new("empty.txt")).unwrap();
        assert!(fd.hunks.is_empty());
        assert_eq!(fd.additions, 0);
        assert!(!fd.binary);
    }

    #[test]
    fn missing_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        assert!(synthesize(dir.path(), Path::new("gone.txt")).is_none());
    }
}

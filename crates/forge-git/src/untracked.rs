//! Synthesizes all-added [`FileDiff`]s for untracked files.

use std::path::Path;

use forge_core::{DiffHunk, DiffLine, FileDiff};

const MAX_LINES: usize = 2000;
const SNIFF_BYTES: usize = 8192;

/// Read an untracked file and present it as a single all-added hunk.
/// `None` for binary files (NUL byte in the first 8 KiB) and unreadable ones.
pub(crate) fn synthesize(repo_root: &Path, rel: &Path) -> Option<FileDiff> {
    let abs = repo_root.join(rel);
    let bytes = match std::fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(path = %abs.display(), error = %e, "skipping unreadable untracked file");
            return None;
        }
    };
    if bytes[..bytes.len().min(SNIFF_BYTES)].contains(&0) {
        return None;
    }

    let text = String::from_utf8_lossy(&bytes);
    let total = text.lines().count();
    let mut lines: Vec<DiffLine> = text
        .lines()
        .take(MAX_LINES)
        .enumerate()
        .map(|(i, l)| DiffLine {
            origin: '+',
            content: l.to_string(),
            old_no: None,
            new_no: Some(i as u32 + 1),
        })
        .collect();
    if total > MAX_LINES {
        lines.push(DiffLine {
            origin: ' ',
            content: format!("… truncated: {} more lines", total - MAX_LINES),
            old_no: None,
            new_no: None,
        });
    }
    let hunks = if lines.is_empty() {
        Vec::new()
    } else {
        vec![DiffHunk { header: format!("@@ -0,0 +1,{total} @@"), lines }]
    };
    Some(FileDiff {
        path: rel.to_string_lossy().into_owned(),
        status: "untracked".into(),
        hunks,
        additions: total as u32,
        deletions: 0,
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
    fn binary_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "bin.dat", b"\x00\x01\x02payload");
        assert!(synthesize(dir.path(), Path::new("bin.dat")).is_none());
    }

    #[test]
    fn long_file_capped_with_marker() {
        let dir = tempfile::tempdir().unwrap();
        let body: String = (1..=2050).map(|i| format!("line {i}\n")).collect();
        write(dir.path(), "big.txt", body.as_bytes());
        let fd = synthesize(dir.path(), Path::new("big.txt")).unwrap();
        assert_eq!(fd.additions, 2050);
        let lines = &fd.hunks[0].lines;
        assert_eq!(lines.len(), 2001);
        assert_eq!(lines[1999].new_no, Some(2000));
        let marker = &lines[2000];
        assert_eq!(marker.origin, ' ');
        assert!(marker.content.contains("truncated: 50 more lines"));
        assert_eq!((marker.old_no, marker.new_no), (None, None));
    }

    #[test]
    fn empty_file_has_no_hunks() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "empty.txt", b"");
        let fd = synthesize(dir.path(), Path::new("empty.txt")).unwrap();
        assert!(fd.hunks.is_empty());
        assert_eq!(fd.additions, 0);
    }

    #[test]
    fn missing_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        assert!(synthesize(dir.path(), Path::new("gone.txt")).is_none());
    }
}

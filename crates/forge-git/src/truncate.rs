//! The single, authoritative diff truncation: cap each file's rendered lines.
//!
//! Applied uniformly to tracked and untracked diffs in `collect_file_diffs`, so
//! there is exactly one place a diff is capped. `FileDiff::truncated` is the
//! source of truth the frontend renders — it does not re-truncate.

use forge_core::FileDiff;

/// Max rendered diff lines per file (summed across hunks). Beyond this the file
/// is capped and `FileDiff::truncated` is set. `additions`/`deletions` still
/// carry the true totals — only the rendered lines are trimmed.
pub(crate) const MAX_DIFF_LINES: usize = 2000;

/// Cap every file at [`MAX_DIFF_LINES`] rendered lines, marking capped files
/// `truncated`.
pub(crate) fn cap_file_diffs(files: &mut [FileDiff]) {
    for file in files {
        cap_one(file, MAX_DIFF_LINES);
    }
}

fn cap_one(file: &mut FileDiff, limit: usize) {
    let total: usize = file.hunks.iter().map(|h| h.lines.len()).sum();
    if total <= limit {
        return;
    }
    let mut budget = limit;
    let mut kept = Vec::new();
    for mut hunk in std::mem::take(&mut file.hunks) {
        if budget == 0 {
            break;
        }
        if hunk.lines.len() > budget {
            hunk.lines.truncate(budget);
        }
        budget -= hunk.lines.len();
        kept.push(hunk);
    }
    file.hunks = kept;
    file.truncated = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{DiffHunk, DiffLine};

    fn hunk(n: usize) -> DiffHunk {
        DiffHunk {
            header: format!("@@ -0,0 +1,{n} @@"),
            lines: (0..n)
                .map(|i| DiffLine {
                    origin: '+',
                    content: format!("l{i}"),
                    old_no: None,
                    new_no: Some(i as u32 + 1),
                })
                .collect(),
        }
    }

    fn file(hunks: Vec<DiffHunk>) -> FileDiff {
        let additions = hunks.iter().map(|h| h.lines.len() as u32).sum();
        FileDiff {
            path: "f".into(),
            status: "added".into(),
            hunks,
            additions,
            deletions: 0,
            binary: false,
            truncated: false,
        }
    }

    #[test]
    fn under_limit_untouched() {
        let mut fs = [file(vec![hunk(10)])];
        cap_file_diffs(&mut fs);
        assert!(!fs[0].truncated);
        assert_eq!(fs[0].hunks[0].lines.len(), 10);
    }

    #[test]
    fn over_limit_capped_and_flagged() {
        let mut fs = [file(vec![hunk(MAX_DIFF_LINES + 500)])];
        cap_file_diffs(&mut fs);
        assert!(fs[0].truncated);
        let shown: usize = fs[0].hunks.iter().map(|h| h.lines.len()).sum();
        assert_eq!(shown, MAX_DIFF_LINES);
        // True total is preserved on additions even though rendering is capped.
        assert_eq!(fs[0].additions, MAX_DIFF_LINES as u32 + 500);
    }

    #[test]
    fn cap_spans_multiple_hunks_dropping_the_tail() {
        // Three hunks of 1500 lines: keep the first whole, cap the second, drop
        // the third entirely.
        let mut fs = [file(vec![hunk(1500), hunk(1500), hunk(1500)])];
        cap_file_diffs(&mut fs);
        assert!(fs[0].truncated);
        assert_eq!(fs[0].hunks.len(), 2);
        assert_eq!(fs[0].hunks[0].lines.len(), 1500);
        assert_eq!(fs[0].hunks[1].lines.len(), MAX_DIFF_LINES - 1500);
    }

    #[test]
    fn exactly_at_limit_not_flagged() {
        let mut fs = [file(vec![hunk(MAX_DIFF_LINES)])];
        cap_file_diffs(&mut fs);
        assert!(!fs[0].truncated);
    }
}

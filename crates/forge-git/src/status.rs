//! `git status --porcelain=v2` parsing.

use std::path::PathBuf;

/// One porcelain-v2 entry, normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusEntry {
    pub path: PathBuf,
    /// Original path for renames/copies.
    pub orig_path: Option<PathBuf>,
    /// "modified" | "added" | "deleted" | "renamed" | "untracked".
    pub status: &'static str,
}

pub(crate) fn parse_porcelain_v2(raw: &str) -> Vec<StatusEntry> {
    raw.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<StatusEntry> {
    let (tag, rest) = line.split_once(' ')?;
    match tag {
        // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
        "1" => {
            let mut fields = rest.splitn(8, ' ');
            let xy = fields.next()?;
            let path = fields.nth(6)?;
            Some(StatusEntry {
                path: PathBuf::from(path),
                orig_path: None,
                status: status_from_xy(xy),
            })
        }
        // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>\t<origPath>
        "2" => {
            let paths = rest.splitn(9, ' ').nth(8)?;
            let (path, orig) = match paths.split_once('\t') {
                Some((p, o)) => (p, Some(o)),
                None => (paths, None),
            };
            Some(StatusEntry {
                path: PathBuf::from(path),
                orig_path: orig.map(PathBuf::from),
                status: "renamed",
            })
        }
        "?" => Some(StatusEntry {
            path: PathBuf::from(rest),
            orig_path: None,
            status: "untracked",
        }),
        // u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
        "u" => Some(StatusEntry {
            path: PathBuf::from(rest.splitn(10, ' ').nth(9)?),
            orig_path: None,
            status: "modified",
        }),
        // "#" headers, "!" ignored entries
        _ => None,
    }
}

fn status_from_xy(xy: &str) -> &'static str {
    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');
    if x == 'D' || y == 'D' {
        "deleted"
    } else if x == 'A' || y == 'A' {
        "added"
    } else if x == 'R' || y == 'R' {
        "renamed"
    } else {
        "modified"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ordinary_entries() {
        let raw = "\
# branch.oid 6934d1d
# branch.head main
1 .M N... 100644 100644 100644 aaaa bbbb src/a.rs
1 A. N... 000000 100644 100644 0000 cccc staged.txt
1 .D N... 100644 100644 000000 dddd eeee gone.txt
? untracked.txt
";
        let entries = parse_porcelain_v2(raw);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].path, PathBuf::from("src/a.rs"));
        assert_eq!(entries[0].status, "modified");
        assert_eq!(entries[1].status, "added");
        assert_eq!(entries[2].status, "deleted");
        assert_eq!(entries[3].status, "untracked");
        assert_eq!(entries[3].path, PathBuf::from("untracked.txt"));
    }

    #[test]
    fn parses_rename_with_orig_path() {
        let raw = "2 R. N... 100644 100644 100644 aaaa bbbb R100 new.txt\told.txt\n";
        let entries = parse_porcelain_v2(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("new.txt"));
        assert_eq!(entries[0].orig_path, Some(PathBuf::from("old.txt")));
        assert_eq!(entries[0].status, "renamed");
    }

    #[test]
    fn parses_unmerged_and_ignores_headers() {
        let raw = "\
# branch.head main
u UU N... 100644 100644 100644 100644 aaaa bbbb cccc conflicted.rs
! build/out.o
";
        let entries = parse_porcelain_v2(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("conflicted.rs"));
        assert_eq!(entries[0].status, "modified");
    }

    #[test]
    fn path_with_spaces() {
        let raw = "1 .M N... 100644 100644 100644 aaaa bbbb src/has space.rs\n";
        let entries = parse_porcelain_v2(raw);
        assert_eq!(entries[0].path, PathBuf::from("src/has space.rs"));
    }
}

//! Hand-rolled unified-diff parser for `git diff --no-color` output.

use forge_core::{DiffHunk, DiffLine, FileDiff};

/// Parse full `git diff` output into per-file [`FileDiff`]s with exact
/// old/new line-number bookkeeping.
pub(crate) fn parse_unified_diff(text: &str) -> Vec<FileDiff> {
    let mut out = Vec::new();
    let mut cur: Option<PendingFile> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(done) = cur.take() {
                out.push(done.finish());
            }
            cur = Some(PendingFile::new(rest));
        } else if let Some(file) = cur.as_mut() {
            file.feed(line);
        }
    }
    if let Some(done) = cur.take() {
        out.push(done.finish());
    }
    out
}

struct PendingFile {
    git_line: String,
    old_path: Option<String>,
    new_path: Option<String>,
    status: &'static str,
    hunks: Vec<DiffHunk>,
    hunk: Option<OpenHunk>,
}

struct OpenHunk {
    header: String,
    lines: Vec<DiffLine>,
    old_no: u32,
    new_no: u32,
    old_left: u32,
    new_left: u32,
}

impl PendingFile {
    fn new(git_line: &str) -> Self {
        Self {
            git_line: git_line.to_string(),
            old_path: None,
            new_path: None,
            status: "modified",
            hunks: Vec::new(),
            hunk: None,
        }
    }

    fn feed(&mut self, line: &str) {
        if let Some((os, oc, ns, nc)) = parse_hunk_header(line) {
            self.close_hunk();
            self.hunk = Some(OpenHunk {
                header: line.to_string(),
                lines: Vec::new(),
                old_no: os,
                new_no: ns,
                old_left: oc,
                new_left: nc,
            });
            return;
        }
        if let Some(h) = self.hunk.as_mut() {
            if h.old_left > 0 || h.new_left > 0 {
                h.push(line);
                return;
            }
            // Hunk counts exhausted: only "\ No newline at end of file" may trail.
            if line.starts_with('\\') {
                return;
            }
            self.close_hunk();
        }
        self.header_line(line);
    }

    fn header_line(&mut self, line: &str) {
        if line.starts_with("new file mode") {
            self.status = "added";
        } else if line.starts_with("deleted file mode") {
            self.status = "deleted";
        } else if let Some(p) = line.strip_prefix("rename from ") {
            self.status = "renamed";
            self.old_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("rename to ") {
            self.status = "renamed";
            self.new_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("copy to ") {
            self.status = "added";
            self.new_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("--- ") {
            if p != "/dev/null" {
                self.old_path = Some(strip_side(p, "a/"));
            }
        } else if let Some(p) = line.strip_prefix("+++ ") {
            if p != "/dev/null" {
                self.new_path = Some(strip_side(p, "b/"));
            }
        }
        // index/mode/similarity lines and "Binary files … differ" carry nothing
        // we render — binary files simply keep zero hunks.
    }

    fn close_hunk(&mut self) {
        if let Some(h) = self.hunk.take() {
            self.hunks.push(DiffHunk { header: h.header, lines: h.lines });
        }
    }

    fn finish(mut self) -> FileDiff {
        self.close_hunk();
        let path = match self.status {
            "deleted" => self.old_path.or(self.new_path),
            _ => self.new_path.or(self.old_path),
        }
        .unwrap_or_else(|| guess_path(&self.git_line));
        let (mut additions, mut deletions) = (0u32, 0u32);
        for hunk in &self.hunks {
            for l in &hunk.lines {
                match l.origin {
                    '+' => additions += 1,
                    '-' => deletions += 1,
                    _ => {}
                }
            }
        }
        FileDiff { path, status: self.status.into(), hunks: self.hunks, additions, deletions }
    }
}

impl OpenHunk {
    fn push(&mut self, line: &str) {
        let (origin, content) = match line.as_bytes().first() {
            Some(b'+') => ('+', &line[1..]),
            Some(b'-') => ('-', &line[1..]),
            Some(b' ') => (' ', &line[1..]),
            Some(b'\\') => return, // "\ No newline at end of file"
            None => (' ', ""),     // be lenient: bare empty context line
            Some(_) => return,     // malformed body line; skip defensively
        };
        let (old_no, new_no) = match origin {
            '+' => {
                let n = self.new_no;
                self.new_no += 1;
                self.new_left = self.new_left.saturating_sub(1);
                (None, Some(n))
            }
            '-' => {
                let n = self.old_no;
                self.old_no += 1;
                self.old_left = self.old_left.saturating_sub(1);
                (Some(n), None)
            }
            _ => {
                let pair = (Some(self.old_no), Some(self.new_no));
                self.old_no += 1;
                self.new_no += 1;
                self.old_left = self.old_left.saturating_sub(1);
                self.new_left = self.new_left.saturating_sub(1);
                pair
            }
        };
        self.lines.push(DiffLine { origin, content: content.to_string(), old_no, new_no });
    }
}

/// `@@ -<old>[,<count>] +<new>[,<count>] @@ …` → (old_start, old_count, new_start, new_count).
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = line.strip_prefix("@@ ")?;
    let ranges = &rest[..rest.find(" @@")?];
    let (old, new) = ranges.split_once(' ')?;
    let (os, oc) = parse_range(old.strip_prefix('-')?)?;
    let (ns, nc) = parse_range(new.strip_prefix('+')?)?;
    Some((os, oc, ns, nc))
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    match s.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((s.parse().ok()?, 1)),
    }
}

/// Strip the `a/` / `b/` diff prefix after unquoting.
fn strip_side(p: &str, prefix: &str) -> String {
    let p = unquote(p);
    match p.strip_prefix(prefix) {
        Some(stripped) => stripped.to_string(),
        None => p,
    }
}

/// Best-effort path from a `diff --git a/<p> b/<p>` line — fallback for
/// sections without `---`/`+++`/rename headers (e.g. mode-only changes).
fn guess_path(git_line: &str) -> String {
    git_line
        .rfind(" b/")
        .map(|i| unquote(&git_line[i + 3..]))
        .unwrap_or_else(|| git_line.to_string())
}

/// Undo git's C-style path quoting (`"pa\ttern"`), including octal byte escapes.
fn unquote(s: &str) -> String {
    let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else {
        return s.to_string();
    };
    let mut bytes: Vec<u8> = Vec::with_capacity(inner.len());
    let mut it = inner.bytes().peekable();
    while let Some(b) = it.next() {
        if b != b'\\' {
            bytes.push(b);
            continue;
        }
        match it.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(d @ b'0'..=b'7') => {
                let mut v = (d - b'0') as u32;
                for _ in 0..2 {
                    match it.peek() {
                        Some(&n @ b'0'..=b'7') => {
                            v = v * 8 + (n - b'0') as u32;
                            it.next();
                        }
                        _ => break,
                    }
                }
                bytes.push(v as u8);
            }
            Some(other) => bytes.push(other),
            None => {}
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

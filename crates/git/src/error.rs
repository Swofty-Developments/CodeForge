//! Error types for git operations.

/// Errors that can occur during git operations.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The specified path is not a git repository.
    #[error("not a git repository: {path}")]
    NotARepository {
        /// The path that was checked.
        path: String,
    },

    /// A git command failed.
    #[error("git command failed: {message}")]
    CommandFailed {
        /// Description of the failure.
        message: String,
        /// The exit code of the git process, if available.
        exit_code: Option<i32>,
    },

    /// An invalid git operation was attempted.
    #[error("invalid operation: {message}")]
    InvalidOperation {
        /// Description of why the operation is invalid.
        message: String,
    },

    /// A merge conflict was encountered.
    #[error("merge conflict in {file_count} file(s)")]
    MergeConflict {
        /// Number of files with conflicts.
        file_count: usize,
    },

    /// An I/O error occurred.
    #[error("io error: {source}")]
    IoError {
        /// The underlying I/O error.
        #[from]
        source: std::io::Error,
    },

    /// A ref (branch, tag) was not found.
    #[error("ref not found: {ref_name}")]
    RefNotFound {
        /// The ref that was not found.
        ref_name: String,
    },
}

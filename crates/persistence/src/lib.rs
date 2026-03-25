use std::path::PathBuf;

/// Stub database handle for the persistence layer.
/// Will be fully implemented in Phase 2.
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

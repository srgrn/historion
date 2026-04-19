use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub cwd: PathBuf,
    pub command: String,
    pub source: EntrySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySource {
    pub file: PathBuf,
    pub line_number: usize,
}

use crate::shell::ShellKind;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordArgs {
    pub cwd: Option<PathBuf>,
    pub command: Option<String>,
    pub history_id: Option<String>,
    pub shell: Option<ShellKind>,
}

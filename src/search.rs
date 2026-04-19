use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArgs {
    pub query: Option<String>,
    pub folder: Option<PathBuf>,
    pub today: bool,
    pub since_days: Option<u32>,
    pub limit: Option<usize>,
    pub json: bool,
}

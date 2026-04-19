use crate::shell::ShellKind;
use std::path::PathBuf;

pub const LOG_DIR_NAME: &str = ".logs";
pub const LOG_FILE_PREFIX: &str = "bash-history-";
pub const LOG_FILE_SUFFIX: &str = ".log";
pub const RECORD_STATE_FILE: &str = ".hy-record-state";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordArgs {
    pub cwd: Option<PathBuf>,
    pub command: Option<String>,
    pub history_id: Option<String>,
    pub shell: Option<ShellKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRequest {
    pub timestamp: String,
    pub cwd: PathBuf,
    pub command: String,
    pub history_id: Option<String>,
    pub shell: Option<ShellKind>,
}

impl RecordArgs {
    pub fn into_request(self, timestamp: String) -> Result<RecordRequest, String> {
        let cwd = self
            .cwd
            .ok_or_else(|| String::from("record requires --cwd"))?;
        let command = self
            .command
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| String::from("record requires --command"))?;

        Ok(RecordRequest {
            timestamp,
            cwd,
            command,
            history_id: self.history_id,
            shell: self.shell,
        })
    }
}

pub fn default_log_dir(home_dir: &std::path::Path) -> PathBuf {
    home_dir.join(LOG_DIR_NAME)
}

pub fn daily_log_path(log_dir: &std::path::Path, date: &str) -> PathBuf {
    log_dir.join(format!("{LOG_FILE_PREFIX}{date}{LOG_FILE_SUFFIX}"))
}

#[cfg(test)]
mod tests {
    use super::{
        LOG_DIR_NAME, LOG_FILE_PREFIX, LOG_FILE_SUFFIX, RecordArgs, RecordRequest, daily_log_path,
        default_log_dir,
    };
    use crate::shell::ShellKind;
    use std::path::{Path, PathBuf};

    #[test]
    fn record_args_require_cwd() {
        let result = RecordArgs {
            cwd: None,
            command: Some(String::from("cargo test")),
            history_id: None,
            shell: None,
        }
        .into_request(String::from("2026-04-19T10:23:45+01:00"));

        assert_eq!(result, Err(String::from("record requires --cwd")));
    }

    #[test]
    fn record_args_require_non_empty_command() {
        let result = RecordArgs {
            cwd: Some(PathBuf::from("/tmp/demo")),
            command: Some(String::from("   ")),
            history_id: None,
            shell: None,
        }
        .into_request(String::from("2026-04-19T10:23:45+01:00"));

        assert_eq!(result, Err(String::from("record requires --command")));
    }

    #[test]
    fn record_args_build_request_with_history_metadata() {
        let request = RecordArgs {
            cwd: Some(PathBuf::from("/tmp/demo")),
            command: Some(String::from("cargo test")),
            history_id: Some(String::from("41")),
            shell: Some(ShellKind::Bash),
        }
        .into_request(String::from("2026-04-19T10:23:45+01:00"))
        .expect("record request should build");

        assert_eq!(
            request,
            RecordRequest {
                timestamp: String::from("2026-04-19T10:23:45+01:00"),
                cwd: PathBuf::from("/tmp/demo"),
                command: String::from("cargo test"),
                history_id: Some(String::from("41")),
                shell: Some(ShellKind::Bash),
            }
        );
    }

    #[test]
    fn default_log_dir_uses_hidden_logs_directory() {
        assert_eq!(
            default_log_dir(Path::new("/home/demo")),
            PathBuf::from(format!("/home/demo/{LOG_DIR_NAME}"))
        );
    }

    #[test]
    fn daily_log_path_uses_daily_file_convention() {
        assert_eq!(
            daily_log_path(Path::new("/home/demo/.logs"), "2026-04-19"),
            PathBuf::from(format!(
                "/home/demo/.logs/{LOG_FILE_PREFIX}2026-04-19{LOG_FILE_SUFFIX}"
            ))
        );
    }
}

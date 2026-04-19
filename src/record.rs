use crate::entry::{escape_field, format_record_line};
use crate::shell::ShellKind;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOG_DIR_NAME: &str = ".logs";
pub const LOG_FILE_PREFIX: &str = "bash-history-";
pub const LOG_FILE_SUFFIX: &str = ".log";
pub const RECORD_STATE_FILE: &str = ".hy-record-state";
pub const LOG_DIR_ENV_VAR: &str = "HY_LOG_DIR";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordOutcome {
    pub log_path: PathBuf,
    pub wrote_entry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordStateKey {
    shell: Option<ShellKind>,
    history_id_escaped: String,
    cwd_escaped: String,
    command_escaped: String,
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

pub fn default_log_dir(home_dir: &Path) -> PathBuf {
    home_dir.join(LOG_DIR_NAME)
}

pub fn resolve_log_dir(home_dir: &Path) -> PathBuf {
    match std::env::var_os(LOG_DIR_ENV_VAR) {
        Some(value) => resolve_log_dir_value(home_dir, &PathBuf::from(value)),
        None => default_log_dir(home_dir),
    }
}

pub fn resolve_log_dir_value(home_dir: &Path, value: &Path) -> PathBuf {
    if value.as_os_str().is_empty() {
        default_log_dir(home_dir)
    } else if value.is_absolute() {
        value.to_path_buf()
    } else {
        home_dir.join(value)
    }
}

pub fn daily_log_path(log_dir: &Path, date: &str) -> PathBuf {
    log_dir.join(format!("{LOG_FILE_PREFIX}{date}{LOG_FILE_SUFFIX}"))
}

pub fn execute(args: RecordArgs) -> Result<RecordOutcome, String> {
    let timestamp = current_timestamp()?;
    let request = args.into_request(timestamp)?;
    let home_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| String::from("HOME is not set"))?;

    append_request(&resolve_log_dir(&home_dir), &request)
}

pub fn append_request(log_dir: &Path, request: &RecordRequest) -> Result<RecordOutcome, String> {
    fs::create_dir_all(log_dir).map_err(|err| err.to_string())?;

    let log_path = daily_log_path(log_dir, date_key(&request.timestamp)?);

    if should_skip_duplicate(log_dir, request)? {
        return Ok(RecordOutcome {
            log_path,
            wrote_entry: false,
        });
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| err.to_string())?;

    let line = format_record_line(&request.timestamp, &request.cwd, &request.command);
    writeln!(file, "{line}").map_err(|err| err.to_string())?;

    write_state_if_needed(log_dir, request)?;

    Ok(RecordOutcome {
        log_path,
        wrote_entry: true,
    })
}

fn current_timestamp() -> Result<String, String> {
    let output = Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S%z")
        .output()
        .map_err(|err| format!("failed to run date: {err}"))?;

    if !output.status.success() {
        return Err(String::from("date command failed"));
    }

    let timestamp = String::from_utf8(output.stdout)
        .map_err(|err| err.to_string())?
        .trim()
        .to_owned();

    if timestamp.is_empty() {
        return Err(String::from("date command returned an empty timestamp"));
    }

    Ok(timestamp)
}

fn date_key(timestamp: &str) -> Result<&str, String> {
    if timestamp.len() < 10 {
        return Err(String::from("timestamp must start with YYYY-MM-DD"));
    }

    Ok(&timestamp[..10])
}

fn should_skip_duplicate(log_dir: &Path, request: &RecordRequest) -> Result<bool, String> {
    let Some(current_key) = RecordStateKey::from_request(request) else {
        return Ok(false);
    };

    let content = match fs::read_to_string(log_dir.join(RECORD_STATE_FILE)) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.to_string()),
    };

    let Some(existing_key) = RecordStateKey::from_line(&content) else {
        return Ok(false);
    };

    Ok(existing_key == current_key)
}

fn write_state_if_needed(log_dir: &Path, request: &RecordRequest) -> Result<(), String> {
    let Some(state) = RecordStateKey::from_request(request) else {
        return Ok(());
    };

    fs::write(
        log_dir.join(RECORD_STATE_FILE),
        format!("{}\n", state.to_line()),
    )
    .map_err(|err| err.to_string())
}

impl RecordStateKey {
    fn from_request(request: &RecordRequest) -> Option<Self> {
        let history_id = request.history_id.as_ref()?;

        Some(Self {
            shell: request.shell,
            history_id_escaped: escape_field(history_id),
            cwd_escaped: escape_field(&request.cwd.to_string_lossy()),
            command_escaped: escape_field(&request.command),
        })
    }

    fn from_line(line: &str) -> Option<Self> {
        let trimmed = line.trim_end();
        let mut parts = trimmed.splitn(4, '\t');
        let shell = match parts.next()? {
            "" => None,
            "bash" => Some(ShellKind::Bash),
            "zsh" => Some(ShellKind::Zsh),
            _ => return None,
        };

        Some(Self {
            shell,
            history_id_escaped: parts.next()?.to_owned(),
            cwd_escaped: parts.next()?.to_owned(),
            command_escaped: parts.next()?.to_owned(),
        })
    }

    fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            shell_name(self.shell),
            self.history_id_escaped,
            self.cwd_escaped,
            self.command_escaped
        )
    }
}

fn shell_name(shell: Option<ShellKind>) -> &'static str {
    match shell {
        Some(ShellKind::Bash) => "bash",
        Some(ShellKind::Zsh) => "zsh",
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LOG_DIR_NAME, LOG_FILE_PREFIX, LOG_FILE_SUFFIX, RECORD_STATE_FILE, RecordArgs,
        RecordOutcome, RecordRequest, append_request, daily_log_path, default_log_dir,
        resolve_log_dir_value,
    };
    use crate::entry::FIELD_DELIMITER;
    use crate::shell::ShellKind;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn resolve_log_dir_value_defaults_for_empty_env() {
        assert_eq!(
            resolve_log_dir_value(Path::new("/home/demo"), Path::new("")),
            PathBuf::from("/home/demo/.logs")
        );
    }

    #[test]
    fn resolve_log_dir_value_supports_absolute_and_relative_paths() {
        assert_eq!(
            resolve_log_dir_value(Path::new("/home/demo"), Path::new("/tmp/hy-logs")),
            PathBuf::from("/tmp/hy-logs")
        );
        assert_eq!(
            resolve_log_dir_value(Path::new("/home/demo"), Path::new("custom-logs")),
            PathBuf::from("/home/demo/custom-logs")
        );
    }

    #[test]
    fn append_request_creates_directory_and_writes_log_line() {
        let temp_dir = make_temp_dir("write-log-line");
        let log_dir = temp_dir.join(".logs");

        let outcome = append_request(
            &log_dir,
            &RecordRequest {
                timestamp: String::from("2026-04-19T10:23:45+0100"),
                cwd: PathBuf::from("/tmp/demo"),
                command: String::from("cargo test"),
                history_id: Some(String::from("41")),
                shell: Some(ShellKind::Zsh),
            },
        )
        .expect("append should succeed");

        assert_eq!(
            outcome,
            RecordOutcome {
                log_path: log_dir.join("bash-history-2026-04-19.log"),
                wrote_entry: true,
            }
        );

        let log_text = fs::read_to_string(log_dir.join("bash-history-2026-04-19.log"))
            .expect("log file should exist");
        assert_eq!(
            log_text,
            format!(
                "2026-04-19T10:23:45+0100{FIELD_DELIMITER}/tmp/demo{FIELD_DELIMITER}cargo test\n"
            )
        );
        assert!(log_dir.join(RECORD_STATE_FILE).exists());

        cleanup(&temp_dir);
    }

    #[test]
    fn append_request_skips_duplicate_history_id() {
        let temp_dir = make_temp_dir("skip-duplicate");
        let log_dir = temp_dir.join(".logs");
        let request = RecordRequest {
            timestamp: String::from("2026-04-19T10:23:45+0100"),
            cwd: PathBuf::from("/tmp/demo"),
            command: String::from("cargo test"),
            history_id: Some(String::from("41")),
            shell: Some(ShellKind::Bash),
        };

        let first = append_request(&log_dir, &request).expect("first append should succeed");
        let second = append_request(&log_dir, &request).expect("second append should succeed");

        assert!(first.wrote_entry);
        assert!(!second.wrote_entry);
        let log_text = fs::read_to_string(log_dir.join("bash-history-2026-04-19.log"))
            .expect("log file should exist");
        assert_eq!(log_text.lines().count(), 1);

        cleanup(&temp_dir);
    }

    #[test]
    fn append_request_writes_when_history_id_changes() {
        let temp_dir = make_temp_dir("different-history-id");
        let log_dir = temp_dir.join(".logs");

        let first = RecordRequest {
            timestamp: String::from("2026-04-19T10:23:45+0100"),
            cwd: PathBuf::from("/tmp/demo"),
            command: String::from("cargo test"),
            history_id: Some(String::from("41")),
            shell: Some(ShellKind::Bash),
        };
        let second = RecordRequest {
            history_id: Some(String::from("42")),
            ..first.clone()
        };

        append_request(&log_dir, &first).expect("first append should succeed");
        append_request(&log_dir, &second).expect("second append should succeed");

        let log_text = fs::read_to_string(log_dir.join("bash-history-2026-04-19.log"))
            .expect("log file should exist");
        assert_eq!(log_text.lines().count(), 2);

        cleanup(&temp_dir);
    }

    #[test]
    fn append_request_without_history_id_does_not_dedupe() {
        let temp_dir = make_temp_dir("no-history-id");
        let log_dir = temp_dir.join(".logs");
        let request = RecordRequest {
            timestamp: String::from("2026-04-19T10:23:45+0100"),
            cwd: PathBuf::from("/tmp/demo"),
            command: String::from("cargo test"),
            history_id: None,
            shell: Some(ShellKind::Zsh),
        };

        append_request(&log_dir, &request).expect("first append should succeed");
        append_request(&log_dir, &request).expect("second append should succeed");

        let log_text = fs::read_to_string(log_dir.join("bash-history-2026-04-19.log"))
            .expect("log file should exist");
        assert_eq!(log_text.lines().count(), 2);
        assert!(!log_dir.join(RECORD_STATE_FILE).exists());

        cleanup(&temp_dir);
    }

    #[test]
    fn append_request_ignores_corrupt_state_file() {
        let temp_dir = make_temp_dir("corrupt-state");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(log_dir.join(RECORD_STATE_FILE), "bad-state").expect("state file should exist");

        let outcome = append_request(
            &log_dir,
            &RecordRequest {
                timestamp: String::from("2026-04-19T10:23:45+0100"),
                cwd: PathBuf::from("/tmp/demo"),
                command: String::from("cargo test"),
                history_id: Some(String::from("41")),
                shell: Some(ShellKind::Bash),
            },
        )
        .expect("append should succeed");

        assert!(outcome.wrote_entry);

        cleanup(&temp_dir);
    }

    fn make_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("hy-tests-{label}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}

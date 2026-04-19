use crate::entry::HistoryEntry;
use crate::output;
use crate::parser;
use crate::record::{LOG_FILE_PREFIX, LOG_FILE_SUFFIX, default_log_dir};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArgs {
    pub query: Option<String>,
    pub folder: Option<PathBuf>,
    pub today: bool,
    pub since_days: Option<u32>,
    pub limit: Option<usize>,
    pub json: bool,
}

pub fn execute(args: SearchArgs, stdout: &mut dyn Write) -> Result<(), String> {
    if args.today || args.since_days.is_some() || args.json {
        return Err(String::from(
            "date filtering and json output are not implemented yet",
        ));
    }

    let home_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| String::from("HOME is not set"))?;
    let log_dir = default_log_dir(&home_dir);
    let entries = search_logs(&log_dir, &args)?;

    stdout
        .write_all(output::render_entries(&entries).as_bytes())
        .map_err(|err| err.to_string())
}

pub fn search_logs(log_dir: &Path, args: &SearchArgs) -> Result<Vec<HistoryEntry>, String> {
    let limit = args.limit.unwrap_or(usize::MAX);
    let mut files = list_log_files(log_dir)?;
    files.sort();
    files.reverse();

    let mut matches = Vec::new();
    let query = args.query.as_deref();

    for file in files {
        let mut file_matches = Vec::new();
        let content = fs::read_to_string(&file).map_err(|err| err.to_string())?;

        for (index, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let Ok(entry) = parser::parse_line(line, &file, index + 1) else {
                continue;
            };

            if matches_query(&entry, query) {
                file_matches.push(entry);
            }
        }

        file_matches.reverse();
        for entry in file_matches {
            matches.push(entry);
            if matches.len() >= limit {
                return Ok(matches);
            }
        }
    }

    Ok(matches)
}

fn list_log_files(log_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.to_string()),
    };

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if path.is_file() && name.starts_with(LOG_FILE_PREFIX) && name.ends_with(LOG_FILE_SUFFIX) {
            files.push(path);
        }
    }

    Ok(files)
}

fn matches_query(entry: &HistoryEntry, query: Option<&str>) -> bool {
    match query {
        Some(query) => entry.command.contains(query),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{SearchArgs, search_logs};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn search_logs_returns_matches_newest_first() {
        let temp_dir = make_temp_dir("search-order");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-18.log"),
            "2026-04-18T10:00:00+0100\t/tmp/demo\tcargo build\n2026-04-18T11:00:00+0100\t/tmp/demo\trg todo\n",
        )
        .expect("older log should be written");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T12:00:00+0100\t/tmp/demo\tcargo test\n",
        )
        .expect("newer log should be written");

        let entries = search_logs(
            &log_dir,
            &SearchArgs {
                query: Some(String::from("cargo")),
                folder: None,
                today: false,
                since_days: None,
                limit: None,
                json: false,
            },
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cargo test");
        assert_eq!(entries[1].command, "cargo build");

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_respects_limit() {
        let temp_dir = make_temp_dir("search-limit");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T09:00:00+0100\t/tmp/demo\tcargo check\n2026-04-19T10:00:00+0100\t/tmp/demo\tcargo test\n",
        )
        .expect("log should be written");

        let entries = search_logs(
            &log_dir,
            &SearchArgs {
                query: Some(String::from("cargo")),
                folder: None,
                today: false,
                since_days: None,
                limit: Some(1),
                json: false,
            },
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo test");

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_skips_malformed_lines_and_supports_legacy_logs() {
        let temp_dir = make_temp_dir("search-legacy");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "bad line\n2026-04-19.10:23:45 /tmp/demo  41  cargo test --lib\n",
        )
        .expect("log should be written");

        let entries = search_logs(
            &log_dir,
            &SearchArgs {
                query: Some(String::from("cargo")),
                folder: None,
                today: false,
                since_days: None,
                limit: None,
                json: false,
            },
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo test --lib");

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

use crate::entry::HistoryEntry;
use crate::output;
use crate::parser;
use crate::record::{LOG_FILE_PREFIX, LOG_FILE_SUFFIX, default_log_dir};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArgs {
    pub query: Option<String>,
    pub folder: Option<PathBuf>,
    pub today: bool,
    pub since_days: Option<u32>,
    pub limit: Option<usize>,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedSearch {
    query: Option<String>,
    folder: Option<String>,
    earliest_date: Option<String>,
    latest_date: Option<String>,
}

impl ResolvedSearch {
    fn from_args(args: &SearchArgs, cwd: &Path, today: Option<&str>) -> Result<Self, String> {
        if args.query.is_none() && args.folder.is_none() {
            return Err(String::from("search requires a query or --folder"));
        }

        let folder = resolve_folder_filter(args.folder.as_deref(), cwd);
        let (earliest_date, latest_date) = match (args.today, args.since_days) {
            (true, _) => {
                let today = today.ok_or_else(|| String::from("today's date is unavailable"))?;
                (Some(today.to_owned()), Some(today.to_owned()))
            }
            (false, Some(days)) => {
                let today = today.ok_or_else(|| String::from("today's date is unavailable"))?;
                (Some(shift_date(today, -(days as i32))?), None)
            }
            (false, None) => (None, None),
        };

        Ok(Self {
            query: args.query.clone(),
            folder,
            earliest_date,
            latest_date,
        })
    }

    fn matches_file_date(&self, file: &Path) -> bool {
        let Some(date) = file_date(file) else {
            return false;
        };

        if let Some(earliest) = self.earliest_date.as_deref() {
            if date < earliest {
                return false;
            }
        }

        if let Some(latest) = self.latest_date.as_deref() {
            if date > latest {
                return false;
            }
        }

        true
    }
}

pub fn execute(args: SearchArgs, stdout: &mut dyn Write) -> Result<(), String> {
    let home_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| String::from("HOME is not set"))?;
    let log_dir = default_log_dir(&home_dir);
    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let entries = search_logs(&log_dir, &args, &cwd)?;

    let rendered = if args.json {
        output::render_entries_as_json(&entries)
    } else {
        output::render_entries(&entries)
    };

    stdout
        .write_all(rendered.as_bytes())
        .map_err(|err| err.to_string())
}

pub fn search_logs(
    log_dir: &Path,
    args: &SearchArgs,
    cwd: &Path,
) -> Result<Vec<HistoryEntry>, String> {
    let today = if args.today || args.since_days.is_some() {
        Some(current_date()?)
    } else {
        None
    };

    search_logs_with_today(log_dir, args, cwd, today.as_deref())
}

pub fn search_logs_with_today(
    log_dir: &Path,
    args: &SearchArgs,
    cwd: &Path,
    today: Option<&str>,
) -> Result<Vec<HistoryEntry>, String> {
    let plan = ResolvedSearch::from_args(args, cwd, today)?;
    let limit = args.limit.unwrap_or(usize::MAX);
    let mut files = list_log_files(log_dir)?;
    files.sort();
    files.reverse();

    let mut matches = Vec::new();

    for file in files {
        if !plan.matches_file_date(&file) {
            continue;
        }

        let mut file_matches = Vec::new();
        let content = fs::read_to_string(&file).map_err(|err| err.to_string())?;

        for (index, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let Ok(entry) = parser::parse_line(line, &file, index + 1) else {
                continue;
            };

            if matches_query(&entry, plan.query.as_deref())
                && matches_folder(&entry, plan.folder.as_deref())
            {
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

fn matches_folder(entry: &HistoryEntry, folder: Option<&str>) -> bool {
    match folder {
        Some(folder) => entry.cwd.to_string_lossy().contains(folder),
        None => true,
    }
}

pub fn resolve_folder_filter(folder: Option<&Path>, cwd: &Path) -> Option<String> {
    folder.map(|folder| {
        if is_path_like(folder) {
            if folder.is_absolute() {
                normalize_path(folder).to_string_lossy().into_owned()
            } else {
                normalize_path(&cwd.join(folder))
                    .to_string_lossy()
                    .into_owned()
            }
        } else {
            folder.to_string_lossy().into_owned()
        }
    })
}

fn is_path_like(path: &Path) -> bool {
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::RootDir | Component::CurDir | Component::ParentDir
            )
        })
        || path.to_string_lossy().contains(std::path::MAIN_SEPARATOR)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn current_date() -> Result<String, String> {
    let output = Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map_err(|err| format!("failed to run date: {err}"))?;

    if !output.status.success() {
        return Err(String::from("date command failed"));
    }

    let date = String::from_utf8(output.stdout)
        .map_err(|err| err.to_string())?
        .trim()
        .to_owned();

    if !is_yyyy_mm_dd(&date) {
        return Err(String::from("date command returned an invalid date"));
    }

    Ok(date)
}

fn file_date(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix(LOG_FILE_PREFIX)?
        .strip_suffix(LOG_FILE_SUFFIX)
}

fn shift_date(date: &str, delta_days: i32) -> Result<String, String> {
    let (year, month, day) = parse_date(date)?;
    let days = days_from_civil(year, month, day) + i64::from(delta_days);
    let (year, month, day) = civil_from_days(days);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn parse_date(date: &str) -> Result<(i32, u32, u32), String> {
    if !is_yyyy_mm_dd(date) {
        return Err(String::from("expected a date in YYYY-MM-DD format"));
    }

    let year = date[0..4]
        .parse::<i32>()
        .map_err(|_| String::from("year must be numeric"))?;
    let month = date[5..7]
        .parse::<u32>()
        .map_err(|_| String::from("month must be numeric"))?;
    let day = date[8..10]
        .parse::<u32>()
        .map_err(|_| String::from("day must be numeric"))?;

    Ok((year, month, day))
}

fn is_yyyy_mm_dd(value: &str) -> bool {
    let bytes = value.as_bytes();

    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i32::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    i64::from(era) * 146097 + i64::from(day_of_era) - 719468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let shifted_days = days + 719468;
    let era = if shifted_days >= 0 {
        shifted_days
    } else {
        shifted_days - 146096
    } / 146097;
    let day_of_era = shifted_days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era as i32 + era as i32 * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };

    (year + i32::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::{
        SearchArgs, resolve_folder_filter, search_logs, search_logs_with_today, shift_date,
    };
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
            Path::new("/work"),
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
            Path::new("/work"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo test");

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_can_filter_to_today_only() {
        let temp_dir = make_temp_dir("search-today");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-18.log"),
            "2026-04-18T10:00:00+0100\t/tmp/demo\tcargo build\n",
        )
        .expect("older log should be written");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T10:00:00+0100\t/tmp/demo\tcargo test\n",
        )
        .expect("today log should be written");

        let entries = search_logs_with_today(
            &log_dir,
            &SearchArgs {
                query: Some(String::from("cargo")),
                folder: None,
                today: true,
                since_days: None,
                limit: None,
                json: false,
            },
            Path::new("/work"),
            Some("2026-04-19"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo test");

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_can_filter_since_a_number_of_days() {
        let temp_dir = make_temp_dir("search-since");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-16.log"),
            "2026-04-16T10:00:00+0100\t/tmp/demo\tcargo build\n",
        )
        .expect("old log should be written");
        fs::write(
            log_dir.join("bash-history-2026-04-18.log"),
            "2026-04-18T10:00:00+0100\t/tmp/demo\tcargo test\n",
        )
        .expect("recent log should be written");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T10:00:00+0100\t/tmp/demo\tcargo run\n",
        )
        .expect("today log should be written");

        let entries = search_logs_with_today(
            &log_dir,
            &SearchArgs {
                query: Some(String::from("cargo")),
                folder: None,
                today: false,
                since_days: Some(1),
                limit: None,
                json: false,
            },
            Path::new("/work"),
            Some("2026-04-19"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cargo run");
        assert_eq!(entries[1].command, "cargo test");

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
            Path::new("/work"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo test --lib");

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_filter_by_folder_prefix() {
        let temp_dir = make_temp_dir("search-folder");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T09:00:00+0100\t/work/project\tcargo check\n2026-04-19T10:00:00+0100\t/work/project/src\trustc main.rs\n2026-04-19T11:00:00+0100\t/work/other\tcargo test\n",
        )
        .expect("log should be written");

        let entries = search_logs(
            &log_dir,
            &SearchArgs {
                query: None,
                folder: Some(PathBuf::from(".")),
                today: false,
                since_days: None,
                limit: None,
                json: false,
            },
            Path::new("/work/project"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].cwd, PathBuf::from("/work/project/src"));
        assert_eq!(entries[1].cwd, PathBuf::from("/work/project"));

        cleanup(&temp_dir);
    }

    #[test]
    fn search_logs_filter_by_partial_folder_name() {
        let temp_dir = make_temp_dir("search-folder-partial");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");
        fs::write(
            log_dir.join("bash-history-2026-04-19.log"),
            "2026-04-19T09:00:00+0100\t/work/project-alpha\tcargo check\n2026-04-19T10:00:00+0100\t/work/project-beta/src\trustc main.rs\n2026-04-19T11:00:00+0100\t/work/other\tcargo test\n",
        )
        .expect("log should be written");

        let entries = search_logs(
            &log_dir,
            &SearchArgs {
                query: None,
                folder: Some(PathBuf::from("project-b")),
                today: false,
                since_days: None,
                limit: None,
                json: false,
            },
            Path::new("/work"),
        )
        .expect("search should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cwd, PathBuf::from("/work/project-beta/src"));

        cleanup(&temp_dir);
    }

    #[test]
    fn resolve_folder_filter_expands_relative_paths() {
        assert_eq!(
            resolve_folder_filter(Some(Path::new(".")), Path::new("/work/project")),
            Some(String::from("/work/project"))
        );
        assert_eq!(
            resolve_folder_filter(Some(Path::new("src/../tests")), Path::new("/work/project")),
            Some(String::from("/work/project/tests"))
        );
        assert_eq!(
            resolve_folder_filter(Some(Path::new("/tmp/demo")), Path::new("/work/project")),
            Some(String::from("/tmp/demo"))
        );
        assert_eq!(
            resolve_folder_filter(Some(Path::new("src")), Path::new("/work/project")),
            Some(String::from("src"))
        );
    }

    #[test]
    fn search_requires_a_query_or_folder() {
        let temp_dir = make_temp_dir("search-validation");
        let log_dir = temp_dir.join(".logs");
        fs::create_dir_all(&log_dir).expect("log dir should exist");

        let result = search_logs_with_today(
            &log_dir,
            &SearchArgs {
                query: None,
                folder: None,
                today: false,
                since_days: None,
                limit: None,
                json: false,
            },
            Path::new("/work"),
            None,
        );

        assert_eq!(
            result,
            Err(String::from("search requires a query or --folder"))
        );

        cleanup(&temp_dir);
    }

    #[test]
    fn shift_date_handles_cross_month_boundaries() {
        assert_eq!(
            shift_date("2026-03-01", -1).expect("date shift should work"),
            "2026-02-28"
        );
        assert_eq!(
            shift_date("2024-03-01", -1).expect("date shift should work"),
            "2024-02-29"
        );
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

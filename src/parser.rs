use crate::entry::{EntrySource, FIELD_DELIMITER, HistoryEntry};
use std::path::{Path, PathBuf};

pub fn parse_line(line: &str, file: &Path, line_number: usize) -> Result<HistoryEntry, String> {
    if line.contains(FIELD_DELIMITER) {
        parse_structured_line(line, file, line_number)
    } else {
        parse_legacy_line(line, file, line_number)
    }
}

pub fn parse_structured_line(
    line: &str,
    file: &Path,
    line_number: usize,
) -> Result<HistoryEntry, String> {
    let mut parts = line.splitn(4, FIELD_DELIMITER);
    let timestamp = parts
        .next()
        .ok_or_else(|| String::from("structured line is missing the timestamp"))?;
    let cwd = parts
        .next()
        .ok_or_else(|| String::from("structured line is missing the cwd"))?;
    let command = parts
        .next()
        .ok_or_else(|| String::from("structured line is missing the command"))?;

    if parts.next().is_some() {
        return Err(String::from("structured line has too many fields"));
    }

    let timestamp = unescape_field(timestamp)?;
    let cwd = unescape_field(cwd)?;
    let command = unescape_field(command)?;

    if timestamp.is_empty() || cwd.is_empty() || command.is_empty() {
        return Err(String::from("structured line contains an empty field"));
    }

    Ok(HistoryEntry {
        timestamp,
        cwd: PathBuf::from(cwd),
        command,
        source: EntrySource {
            file: file.to_path_buf(),
            line_number,
        },
    })
}

pub fn parse_legacy_line(
    line: &str,
    file: &Path,
    line_number: usize,
) -> Result<HistoryEntry, String> {
    let line = line.trim_end();
    let timestamp = line
        .get(..19)
        .ok_or_else(|| String::from("legacy line is too short"))?;

    if !looks_like_legacy_timestamp(timestamp) {
        return Err(String::from(
            "legacy line does not start with YYYY-MM-DD.HH:MM:SS",
        ));
    }

    let remainder = line
        .get(20..)
        .ok_or_else(|| String::from("legacy line is missing cwd and command"))?;

    let (cwd, command) = split_legacy_remainder(remainder)?;

    Ok(HistoryEntry {
        timestamp: timestamp.to_owned(),
        cwd: PathBuf::from(cwd),
        command,
        source: EntrySource {
            file: file.to_path_buf(),
            line_number,
        },
    })
}

pub fn unescape_field(value: &str) -> Result<String, String> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        let escaped = chars
            .next()
            .ok_or_else(|| String::from("field ends with a trailing escape"))?;

        match escaped {
            '\\' => result.push('\\'),
            't' => result.push('\t'),
            'n' => result.push('\n'),
            other => {
                result.push('\\');
                result.push(other);
            }
        }
    }

    Ok(result)
}

fn looks_like_legacy_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();

    bytes.len() == 19
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'.'
        && bytes[13] == b':'
        && bytes[16] == b':'
}

fn split_legacy_remainder(remainder: &str) -> Result<(String, String), String> {
    let mut candidate = None;
    let bytes = remainder.as_bytes();

    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_whitespace() {
            continue;
        }

        let mut cursor = start;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let digit_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if cursor == digit_start {
            continue;
        }

        let whitespace_after_digits = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        if cursor == whitespace_after_digits {
            continue;
        }

        let cwd = remainder[..start].trim_end();
        if cwd.starts_with('/') || cwd == "." {
            candidate = Some((cwd.to_owned(), remainder[cursor..].to_owned()));
            break;
        }
    }

    candidate.ok_or_else(|| String::from("legacy line is missing a recognizable history marker"))
}

#[cfg(test)]
mod tests {
    use super::{parse_legacy_line, parse_line, parse_structured_line, unescape_field};
    use crate::entry::{HistoryEntry, escape_field};
    use std::path::{Path, PathBuf};

    #[test]
    fn structured_lines_round_trip() {
        let line = format!(
            "{}\t{}\t{}",
            escape_field("2026-04-19T10:23:45+0100"),
            escape_field("/tmp/demo project"),
            escape_field("printf 'a\tb'\n")
        );

        let entry = parse_structured_line(&line, Path::new("/tmp/log.log"), 12)
            .expect("structured line should parse");

        assert_eq!(
            entry,
            HistoryEntry {
                timestamp: String::from("2026-04-19T10:23:45+0100"),
                cwd: PathBuf::from("/tmp/demo project"),
                command: String::from("printf 'a\tb'\n"),
                source: crate::entry::EntrySource {
                    file: PathBuf::from("/tmp/log.log"),
                    line_number: 12,
                },
            }
        );
    }

    #[test]
    fn structured_lines_require_three_fields() {
        let result = parse_structured_line(
            "2026-04-19T10:23:45+0100\t/tmp/demo",
            Path::new("/tmp/log.log"),
            3,
        );

        assert_eq!(
            result,
            Err(String::from("structured line is missing the command"))
        );
    }

    #[test]
    fn unescape_field_restores_special_characters() {
        let result =
            unescape_field("first\\tsecond\\nthird\\\\fourth").expect("field should unescape");

        assert_eq!(result, "first\tsecond\nthird\\fourth");
    }

    #[test]
    fn legacy_lines_parse_history_number_and_command() {
        let entry = parse_legacy_line(
            "2026-04-19.10:23:45 /tmp/demo  41  cargo test --lib",
            Path::new("/tmp/legacy.log"),
            8,
        )
        .expect("legacy line should parse");

        assert_eq!(entry.timestamp, "2026-04-19.10:23:45");
        assert_eq!(entry.cwd, PathBuf::from("/tmp/demo"));
        assert_eq!(entry.command, "cargo test --lib");
        assert_eq!(entry.source.line_number, 8);
    }

    #[test]
    fn legacy_lines_support_spaces_in_path() {
        let entry = parse_legacy_line(
            "2026-04-19.10:23:45 /tmp/demo project  41  cargo test",
            Path::new("/tmp/legacy.log"),
            9,
        )
        .expect("legacy line should parse");

        assert_eq!(entry.cwd, PathBuf::from("/tmp/demo project"));
        assert_eq!(entry.command, "cargo test");
    }

    #[test]
    fn legacy_lines_fail_without_history_marker() {
        let result = parse_legacy_line(
            "2026-04-19.10:23:45 /tmp/demo cargo test",
            Path::new("/tmp/legacy.log"),
            4,
        );

        assert_eq!(
            result,
            Err(String::from(
                "legacy line is missing a recognizable history marker"
            ))
        );
    }

    #[test]
    fn parse_line_auto_detects_structured_and_legacy() {
        let structured = parse_line(
            "2026-04-19T10:23:45+0100\t/tmp/demo\tcargo test",
            Path::new("/tmp/structured.log"),
            1,
        )
        .expect("structured line should parse");
        let legacy = parse_line(
            "2026-04-19.10:23:45 /tmp/demo  41  cargo test",
            Path::new("/tmp/legacy.log"),
            2,
        )
        .expect("legacy line should parse");

        assert_eq!(structured.command, "cargo test");
        assert_eq!(legacy.command, "cargo test");
    }
}

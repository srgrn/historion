use std::path::{Path, PathBuf};

pub const FIELD_DELIMITER: char = '\t';

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

impl HistoryEntry {
    pub fn format_escaped_tsv(&self) -> String {
        format_record_line(&self.timestamp, &self.cwd, &self.command)
    }
}

pub fn format_record_line(timestamp: &str, cwd: &Path, command: &str) -> String {
    format!(
        "{}{FIELD_DELIMITER}{}{FIELD_DELIMITER}{}",
        escape_field(timestamp),
        escape_field(&cwd.to_string_lossy()),
        escape_field(command)
    )
}

pub fn escape_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::{EntrySource, FIELD_DELIMITER, HistoryEntry, escape_field, format_record_line};
    use std::path::{Path, PathBuf};

    #[test]
    fn escape_field_preserves_plain_text() {
        assert_eq!(escape_field("cargo test"), "cargo test");
    }

    #[test]
    fn escape_field_encodes_special_characters() {
        assert_eq!(
            escape_field("first\tsecond\nthird\\fourth"),
            "first\\tsecond\\nthird\\\\fourth"
        );
    }

    #[test]
    fn history_entry_formats_as_escaped_tsv() {
        let entry = HistoryEntry {
            timestamp: String::from("2026-04-19T10:23:45+01:00"),
            cwd: PathBuf::from("/tmp/project with spaces"),
            command: String::from("printf 'a\tb'\n"),
            source: EntrySource {
                file: PathBuf::from("/tmp/source.log"),
                line_number: 7,
            },
        };

        let line = entry.format_escaped_tsv();

        assert_eq!(
            line,
            format!(
                "2026-04-19T10:23:45+01:00{FIELD_DELIMITER}/tmp/project with spaces{FIELD_DELIMITER}printf 'a\\tb'\\n"
            )
        );
    }

    #[test]
    fn format_record_line_matches_expected_layout() {
        let line = format_record_line(
            "2026-04-19T10:23:45+01:00",
            Path::new("/tmp/demo"),
            "cargo test",
        );

        assert_eq!(
            line,
            format!(
                "2026-04-19T10:23:45+01:00{FIELD_DELIMITER}/tmp/demo{FIELD_DELIMITER}cargo test"
            )
        );
    }
}

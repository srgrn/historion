use crate::entry::HistoryEntry;

pub fn help_text() -> &'static str {
    r#"Usage:
  hy <query> [--folder PATH] [--today] [--since DAYS] [--limit N] [--json]
  hy record --cwd PATH --command COMMAND [--history-id ID] [--shell bash|zsh]
  hy init <bash|zsh>
  hy install <bash|zsh>

Commands:
  <query>   Search command history for a substring
  record    Append a shell command to the daily history log
  init      Print shell integration for bash or zsh
  install   Install shell integration into the rc file
"#
}

pub fn render_entries(entries: &[HistoryEntry]) -> String {
    let mut output = String::new();

    for entry in entries {
        output.push_str(&entry.format_escaped_tsv());
        output.push('\n');
    }

    output
}

pub fn render_entries_as_json(entries: &[HistoryEntry]) -> String {
    let mut output = String::from("[");

    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }

        output.push_str("{\"timestamp\":\"");
        output.push_str(&escape_json(&entry.timestamp));
        output.push_str("\",\"cwd\":\"");
        output.push_str(&escape_json(&entry.cwd.to_string_lossy()));
        output.push_str("\",\"command\":\"");
        output.push_str(&escape_json(&entry.command));
        output.push_str("\",\"file\":\"");
        output.push_str(&escape_json(&entry.source.file.to_string_lossy()));
        output.push_str("\",\"line\":");
        output.push_str(&entry.source.line_number.to_string());
        output.push('}');
    }

    output.push_str("]\n");
    output
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::{render_entries, render_entries_as_json};
    use crate::entry::{EntrySource, HistoryEntry};
    use std::path::PathBuf;

    #[test]
    fn render_entries_uses_one_line_per_entry() {
        let entries = vec![HistoryEntry {
            timestamp: String::from("2026-04-19T10:23:45+0100"),
            cwd: PathBuf::from("/tmp/demo"),
            command: String::from("cargo test"),
            source: EntrySource {
                file: PathBuf::from("/tmp/log.log"),
                line_number: 1,
            },
        }];

        assert_eq!(
            render_entries(&entries),
            "2026-04-19T10:23:45+0100\t/tmp/demo\tcargo test\n"
        );
    }

    #[test]
    fn render_entries_as_json_returns_machine_readable_output() {
        let entries = vec![HistoryEntry {
            timestamp: String::from("2026-04-19T10:23:45+0100"),
            cwd: PathBuf::from("/tmp/demo"),
            command: String::from("printf \"hi\"\n"),
            source: EntrySource {
                file: PathBuf::from("/tmp/log.log"),
                line_number: 3,
            },
        }];

        assert_eq!(
            render_entries_as_json(&entries),
            "[{\"timestamp\":\"2026-04-19T10:23:45+0100\",\"cwd\":\"/tmp/demo\",\"command\":\"printf \\\"hi\\\"\\n\",\"file\":\"/tmp/log.log\",\"line\":3}]\n"
        );
    }
}

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

#[cfg(test)]
mod tests {
    use super::render_entries;
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
}

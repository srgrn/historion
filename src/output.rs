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

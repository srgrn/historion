# hy

`hy` is a planned Rust CLI for recording and searching shell command history stored in plain text log files.

The goal is to keep the logs simple enough for `grep`, `awk`, or manual inspection, while providing a better interactive search tool for day-to-day use.

## Goals

- Keep history in text files under `~/.logs/`
- Support fast direct search without requiring a database in v1
- Replace custom per-machine shell snippets with a standard `hy` integration flow
- Preserve enough structure for reliable filtering by command text and working directory

## Planned commands

- `hy record`: append the latest shell command to the daily log file
- `hy <query>`: search command text
- `hy --folder <path>`: filter results by working directory
- `hy init zsh|bash`: print the minimal shell hook
- `hy install zsh|bash`: install the managed shell hook

## Log format

The preferred format is plain text with escaped tab-separated fields:

```text
2026-04-19T10:23:45+01:00	/home/zimbl/project	cargo test --lib
2026-04-19T10:25:02+01:00	/home/zimbl/project/src	rg history
```

Storage rules:

- One entry per line
- Three logical fields: timestamp, current working directory, command
- Tabs, newlines, and backslashes inside field values are escaped
- Files remain grep-friendly and human-readable

## Design notes

- A shell hook is still required because the shell is the component that knows which command just ran.
- The hook should be minimal and call `hy record` rather than containing logging logic inline.
- Legacy space-delimited history logs should be supported on a best-effort basis for migration.
- Folder filtering should treat `--folder .` as the current directory tree.

## Development status

This repository is currently in the planning stage. See `tasks.md` for the working task breakdown and `LESSONS.md` for decisions and implementation notes that should persist across tasks.


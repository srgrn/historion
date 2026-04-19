# LESSONS

This file is a shared memory for implementation tasks. It should capture stable decisions, preferences, sharp edges, and project structure notes that future work should reuse.

## Current decisions

- The tool name is `hy`.
- The project is a Rust CLI, starting as a single binary crate.
- The crate should stay dependency-free unless a later task proves a strong need for external crates.
- Logs must remain plain text so they are still usable with `grep` and similar tools.
- The preferred on-disk format is escaped TSV, not JSON and not a database.
- Daily files should live under `~/.logs/` with names like `bash-history-YYYY-MM-DD.log`.
- `hy` should replace custom logging logic in shell snippets, but it cannot eliminate shell integration entirely.

## Shell integration gotcha

- Bash and zsh know what command just ran; a standalone binary does not.
- Because of that, `hy` can centralize the logic, but it still needs either:
  - a minimal shell hook that calls `hy record`, or
  - an instrumented shell mode such as `hy shell` later

## Parsing gotcha

- The original space-delimited format is ambiguous when paths or commands contain spaces.
- Exact folder matching depends on adopting the structured escaped-TSV format.
- Legacy logs should be treated as migration input with best-effort parsing only.

## Record contract

- The searchable log line stores exactly three escaped-TSV fields: timestamp, cwd, and command.
- `history_id` is useful for duplicate suppression, but it should not be written into the searchable log line.
- Duplicate suppression state can live in a separate hidden text file inside the log directory.
- `hy record` should reject missing `--cwd` and empty `--command` values before touching the filesystem.

## Search semantics

- `hy <query>` should search command text by substring in v1.
- `hy --folder <path>` should resolve relative paths from the caller's current working directory.
- Folder matches should be recursive prefix matches by default.
- `hy --folder .` should mean "current directory and descendants".

## Scope guardrails

- No database in v1.
- Directly scan daily log files first; optimize only if real usage demands it.
- Keep output human-readable by default.
- Move logic into Rust and keep shell snippets as thin as possible.

## Repository notes

- `tasks.md` is intentionally local-only and must not be committed.
- `README.md` should describe public behavior and setup.
- `LESSONS.md` should record decisions that future tasks should not rediscover.
- Keep the crate split into `lib.rs` plus a thin `main.rs` so command parsing and behavior remain easy to unit test.

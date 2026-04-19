# hy

`hy` is a small Rust CLI for recording and searching shell history in plain text log files under `~/.logs/`.

It is meant to replace ad-hoc `precmd` or `PROMPT_COMMAND` snippets with a stable `hy`-managed hook while keeping the history files easy to inspect with `grep`, `awk`, or `sed`.

## Features

- Records shell commands into daily text files such as `~/.logs/bash-history-2026-04-19.log`
- Searches command text with `hy <query>`
- Filters by working directory tree with `hy --folder <path>`
- Supports `--today`, `--since <days>`, `--limit <n>`, and `--json`
- Installs managed shell hooks with `hy install bash` or `hy install zsh`
- Reads the newer escaped-TSV format exactly and older space-delimited logs on a best-effort basis

## Install

Build locally:

```bash
cargo build
```

Install into Cargo's bin directory:

```bash
cargo install --path .
```

If you do not want `hy` on your `PATH`, set `HY_BIN` in your shell before loading the hook:

```bash
export HY_BIN="$HOME/.cargo/bin/hy"
```

## Shell Setup

Preview the generated hook:

```bash
hy init zsh
hy init bash
```

Install the managed hook into your rc file:

```bash
hy install zsh
hy install bash
```

`hy install` writes a marked block into `~/.zshrc` or `~/.bashrc` and updates that same block if you run it again. It does not need the old inline history snippet.

## Usage

Search for commands containing a word:

```bash
hy cargo
```

Search only within the current directory tree:

```bash
hy --folder .
```

Combine text search and folder filtering:

```bash
hy cargo --folder .
```

Limit to recent logs:

```bash
hy cargo --today
hy cargo --since 7
```

Get machine-readable output:

```bash
hy cargo --json
```

`record` is meant for shell hooks, but it can also be called directly:

```bash
hy record --cwd "$PWD" --command "cargo test" --history-id 42 --shell zsh
```

## Log Format

`hy` stores one escaped-TSV record per line:

```text
2026-04-19T10:23:45+0100	/home/zimbl/project	cargo test --lib
2026-04-19T10:25:02+0100	/home/zimbl/project/src	rg history
```

Rules:

- Field order is `timestamp<TAB>cwd<TAB>command`
- Tabs, newlines, and backslashes inside values are escaped
- Files stay readable and grep-friendly
- Duplicate suppression metadata is kept in a separate hidden state file, not inside the searchable log line

You can still use plain shell tools directly:

```bash
grep cargo ~/.logs/bash-history-*.log
```

## Migration From Old Snippets

If you previously had a custom `precmd` or `PROMPT_COMMAND` snippet that wrote directly to `~/.logs`, the migration path is:

1. Remove the old shell snippet from your rc file.
2. Install `hy`.
3. Run `hy install zsh` or `hy install bash`.
4. Reload your shell.

Old space-delimited log files are still searchable, but only on a best-effort basis because the original format is ambiguous when paths or commands contain spaces. New `hy`-written logs use the structured escaped-TSV format and are parsed exactly.

## Development

Run the full test suite:

```bash
cargo test
```

The repository also keeps a `LESSONS.md` file with implementation decisions and gotchas that should persist across future tasks.

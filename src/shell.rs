use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Zsh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitArgs {
    pub shell: ShellKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallArgs {
    pub shell: ShellKind,
}

pub fn execute_init(args: InitArgs, stdout: &mut dyn Write) -> Result<(), String> {
    stdout
        .write_all(managed_block(args.shell).as_bytes())
        .map_err(|err| err.to_string())
}

pub fn execute_install(args: InstallArgs, stdout: &mut dyn Write) -> Result<(), String> {
    let home_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| String::from("HOME is not set"))?;
    let rc_path = rc_path(args.shell, &home_dir);
    let changed = install_into_file(&rc_path, args.shell)?;

    let message = if changed {
        format!("installed {}\n", rc_path.display())
    } else {
        format!("already installed {}\n", rc_path.display())
    };

    stdout
        .write_all(message.as_bytes())
        .map_err(|err| err.to_string())
}

pub fn install_into_file(path: &Path, shell: ShellKind) -> Result<bool, String> {
    let current = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.to_string()),
    };

    let updated = upsert_managed_block(&current, shell);
    if updated == current {
        return Ok(false);
    }

    fs::write(path, updated).map_err(|err| err.to_string())?;
    Ok(true)
}

pub fn rc_path(shell: ShellKind, home_dir: &Path) -> PathBuf {
    home_dir.join(match shell {
        ShellKind::Bash => ".bashrc",
        ShellKind::Zsh => ".zshrc",
    })
}

fn upsert_managed_block(existing: &str, shell: ShellKind) -> String {
    let block = managed_block(shell);
    let (start_marker, end_marker) = managed_markers(shell);

    if let Some(start) = existing.find(start_marker) {
        if let Some(end_offset) = existing[start..].find(end_marker) {
            let mut end = start + end_offset + end_marker.len();
            if existing[end..].starts_with("\r\n") {
                end += 2;
            } else if existing[end..].starts_with('\n') {
                end += 1;
            }
            let mut updated = String::new();
            updated.push_str(&existing[..start]);
            updated.push_str(&block);
            updated.push_str(&existing[end..]);
            return normalize_trailing_newline(updated);
        }
    }

    if existing.trim().is_empty() {
        return block;
    }

    let mut updated = String::from(existing.trim_end_matches('\n'));
    updated.push_str("\n\n");
    updated.push_str(&block);
    normalize_trailing_newline(updated)
}

fn managed_block(shell: ShellKind) -> String {
    let (start_marker, end_marker) = managed_markers(shell);
    format!("{start_marker}\n{}{end_marker}\n", shell_snippet(shell))
}

fn managed_markers(shell: ShellKind) -> (&'static str, &'static str) {
    match shell {
        ShellKind::Bash => (
            "# >>> hy bash integration >>>",
            "# <<< hy bash integration <<<",
        ),
        ShellKind::Zsh => (
            "# >>> hy zsh integration >>>",
            "# <<< hy zsh integration <<<",
        ),
    }
}

fn shell_snippet(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Bash => BASH_SNIPPET,
        ShellKind::Zsh => ZSH_SNIPPET,
    }
}

fn normalize_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }

    text
}

const BASH_SNIPPET: &str = r#"__hy_prompt_command() {
    [ "$(id -u)" -eq 0 ] && return

    local __hy_line __hy_cmd __hy_hist_id
    __hy_line=$(builtin history 1)
    __hy_cmd=$(printf '%s\n' "$__hy_line" | sed 's/^[[:space:]]*[0-9]\+[[:space:]]*//')
    __hy_hist_id=$(printf '%s\n' "$__hy_line" | sed 's/^[[:space:]]*//; s/[[:space:]].*$//')

    [ -z "$__hy_cmd" ] && return

    "${HY_BIN:-hy}" record --cwd "$PWD" --command "$__hy_cmd" --history-id "$__hy_hist_id" --shell bash >/dev/null 2>&1 || true
}

case ";${PROMPT_COMMAND-};" in
    *";__hy_prompt_command;"*) ;;
    "") PROMPT_COMMAND="__hy_prompt_command" ;;
    *) PROMPT_COMMAND="__hy_prompt_command;${PROMPT_COMMAND}" ;;
esac
"#;

const ZSH_SNIPPET: &str = r#"__hy_precmd() {
    [ "$(id -u)" -eq 0 ] && return

    local __hy_line __hy_cmd __hy_hist_id
    __hy_line=$(fc -l -1)
    __hy_cmd=$(fc -ln -1)
    __hy_hist_id=$(printf '%s\n' "$__hy_line" | sed 's/^[[:space:]]*//; s/[[:space:]].*$//')

    [ -z "$__hy_cmd" ] && return

    "${HY_BIN:-hy}" record --cwd "$PWD" --command "$__hy_cmd" --history-id "$__hy_hist_id" --shell zsh >/dev/null 2>&1 || true
}

autoload -Uz add-zsh-hook 2>/dev/null
if typeset -f add-zsh-hook >/dev/null 2>&1; then
    add-zsh-hook precmd __hy_precmd
else
    case " ${precmd_functions[*]-} " in
        *" __hy_precmd "*) ;;
        *) precmd_functions+=(__hy_precmd) ;;
    esac
fi
"#;

#[cfg(test)]
mod tests {
    use super::{ShellKind, install_into_file, managed_block, rc_path};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn managed_block_contains_expected_command() {
        let block = managed_block(ShellKind::Zsh);

        assert!(block.contains("${HY_BIN:-hy}"));
        assert!(block.contains("record --cwd"));
        assert!(block.contains("--shell zsh"));
        assert!(block.contains("# >>> hy zsh integration >>>"));
    }

    #[test]
    fn install_into_file_creates_rc_file_with_managed_block() {
        let temp_dir = make_temp_dir("shell-install-create");
        let rc_file = temp_dir.join(".bashrc");

        let changed = install_into_file(&rc_file, ShellKind::Bash).expect("install should work");

        assert!(changed);
        let content = fs::read_to_string(&rc_file).expect("rc file should exist");
        assert!(content.contains("# >>> hy bash integration >>>"));
        assert!(content.contains("PROMPT_COMMAND"));

        cleanup(&temp_dir);
    }

    #[test]
    fn install_into_file_is_idempotent() {
        let temp_dir = make_temp_dir("shell-install-idempotent");
        let rc_file = temp_dir.join(".zshrc");

        assert!(install_into_file(&rc_file, ShellKind::Zsh).expect("first install should work"));
        assert!(!install_into_file(&rc_file, ShellKind::Zsh).expect("second install should work"));

        let content = fs::read_to_string(&rc_file).expect("rc file should exist");
        assert_eq!(content.matches("# >>> hy zsh integration >>>").count(), 1);

        cleanup(&temp_dir);
    }

    #[test]
    fn install_into_file_replaces_existing_managed_block() {
        let temp_dir = make_temp_dir("shell-install-replace");
        let rc_file = temp_dir.join(".bashrc");
        fs::write(
            &rc_file,
            "export PATH=\"$HOME/bin:$PATH\"\n# >>> hy bash integration >>>\noutdated\n# <<< hy bash integration <<<\n",
        )
        .expect("rc file should be seeded");

        let changed = install_into_file(&rc_file, ShellKind::Bash).expect("install should work");
        let content = fs::read_to_string(&rc_file).expect("rc file should exist");

        assert!(changed);
        assert!(content.starts_with("export PATH=\"$HOME/bin:$PATH\""));
        assert!(!content.contains("outdated"));
        assert_eq!(content.matches("# >>> hy bash integration >>>").count(), 1);

        cleanup(&temp_dir);
    }

    #[test]
    fn rc_path_matches_shell_conventions() {
        assert_eq!(
            rc_path(ShellKind::Bash, Path::new("/home/demo")),
            PathBuf::from("/home/demo/.bashrc")
        );
        assert_eq!(
            rc_path(ShellKind::Zsh, Path::new("/home/demo")),
            PathBuf::from("/home/demo/.zshrc")
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

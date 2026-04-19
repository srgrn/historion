use crate::record::RecordArgs;
use crate::search::SearchArgs;
use crate::shell::{InitArgs, InstallArgs, ShellKind};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Search(SearchArgs),
    Record(RecordArgs),
    Init(InitArgs),
    Install(InstallArgs),
}

pub fn parse_args<I, T>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args = args
        .into_iter()
        .map(|item| {
            item.into()
                .into_string()
                .map_err(|_| String::from("arguments must be valid unicode"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut args = args.into_iter();
    let _program = args.next();

    let Some(first) = args.next() else {
        return Ok(Command::Help);
    };

    match first.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "record" => parse_record(args.collect()),
        "init" => parse_init(args.collect()),
        "install" => parse_install(args.collect()),
        value if value.starts_with("--") => {
            parse_search(None, std::iter::once(first).chain(args).collect())
        }
        _ => parse_search(Some(first), args.collect()),
    }
}

fn parse_search(query: Option<String>, rest: Vec<String>) -> Result<Command, String> {
    let mut query = query;
    let mut folder = None;
    let mut today = false;
    let mut since = None;
    let mut limit = None;
    let mut json = false;
    let mut ignore_case = false;

    let mut rest = rest.into_iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--folder" => {
                let value = rest
                    .next()
                    .ok_or_else(|| String::from("--folder requires a path"))?;
                folder = Some(PathBuf::from(value));
            }
            "--today" => today = true,
            "--since" => {
                let value = rest
                    .next()
                    .ok_or_else(|| String::from("--since requires a number of days"))?;
                since = Some(
                    value
                        .parse()
                        .map_err(|_| String::from("--since expects an integer"))?,
                );
            }
            "--limit" => {
                let value = rest
                    .next()
                    .ok_or_else(|| String::from("--limit requires a number"))?;
                limit = Some(
                    value
                        .parse()
                        .map_err(|_| String::from("--limit expects an integer"))?,
                );
            }
            "-i" | "--ignore-case" => ignore_case = true,
            "--json" => json = true,
            "-h" | "--help" => return Ok(Command::Help),
            value if value.starts_with("--") => {
                return Err(format!("unknown search flag: {value}"));
            }
            value => {
                if query.is_none() {
                    query = Some(value.to_owned());
                } else {
                    return Err(String::from("search accepts only one query argument"));
                }
            }
        }
    }

    Ok(Command::Search(SearchArgs {
        query,
        folder,
        today,
        since_days: since,
        limit,
        json,
        ignore_case,
    }))
}

fn parse_record(args: Vec<String>) -> Result<Command, String> {
    let mut cwd = None;
    let mut command = None;
    let mut history_id = None;
    let mut shell = None;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cwd" => {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("--cwd requires a path"))?;
                cwd = Some(PathBuf::from(value));
            }
            "--command" => {
                command = Some(
                    args.next()
                        .ok_or_else(|| String::from("--command requires a value"))?,
                );
            }
            "--history-id" => {
                history_id = Some(
                    args.next()
                        .ok_or_else(|| String::from("--history-id requires a value"))?,
                );
            }
            "--shell" => {
                shell = Some(parse_shell_kind(
                    &args
                        .next()
                        .ok_or_else(|| String::from("--shell requires a value"))?,
                )?);
            }
            value => return Err(format!("unknown record flag: {value}")),
        }
    }

    Ok(Command::Record(RecordArgs {
        cwd,
        command,
        history_id,
        shell,
    }))
}

fn parse_init(args: Vec<String>) -> Result<Command, String> {
    let shell = parse_single_shell_arg("init", args)?;
    Ok(Command::Init(InitArgs { shell }))
}

fn parse_install(args: Vec<String>) -> Result<Command, String> {
    let shell = parse_single_shell_arg("install", args)?;
    Ok(Command::Install(InstallArgs { shell }))
}

fn parse_single_shell_arg(command: &str, args: Vec<String>) -> Result<ShellKind, String> {
    let mut args = args.into_iter();
    let value = args
        .next()
        .ok_or_else(|| format!("{command} requires a shell argument"))?;

    if args.next().is_some() {
        return Err(format!("{command} accepts only one shell argument"));
    }

    parse_shell_kind(&value)
}

fn parse_shell_kind(value: &str) -> Result<ShellKind, String> {
    match value {
        "bash" => Ok(ShellKind::Bash),
        "zsh" => Ok(ShellKind::Zsh),
        _ => Err(format!("unsupported shell: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_args};
    use crate::record::RecordArgs;
    use crate::search::SearchArgs;
    use crate::shell::{InitArgs, InstallArgs, ShellKind};
    use std::path::PathBuf;

    #[test]
    fn parses_help_without_arguments() {
        let command = parse_args(["hy"]).expect("cli should parse");
        assert_eq!(command, Command::Help);
    }

    #[test]
    fn parses_search_command_from_bare_query() {
        let command = parse_args(["hy", "needle"]).expect("cli should parse");
        assert_eq!(
            command,
            Command::Search(SearchArgs {
                query: Some(String::from("needle")),
                folder: None,
                today: false,
                since_days: None,
                limit: None,
                json: false,
                ignore_case: false,
            })
        );
    }

    #[test]
    fn parses_search_flags_without_a_query() {
        let command = parse_args(["hy", "--folder", "."]).expect("cli should parse");

        assert_eq!(
            command,
            Command::Search(SearchArgs {
                query: None,
                folder: Some(PathBuf::from(".")),
                today: false,
                since_days: None,
                limit: None,
                json: false,
                ignore_case: false,
            })
        );
    }

    #[test]
    fn parses_search_query_after_folder_flag() {
        let command = parse_args(["hy", "--folder", ".", "cargo"]).expect("cli should parse");

        assert_eq!(
            command,
            Command::Search(SearchArgs {
                query: Some(String::from("cargo")),
                folder: Some(PathBuf::from(".")),
                today: false,
                since_days: None,
                limit: None,
                json: false,
                ignore_case: false,
            })
        );
    }

    #[test]
    fn parses_ignore_case_flag() {
        let command =
            parse_args(["hy", "--folder", ".", "--ignore-case"]).expect("cli should parse");

        assert_eq!(
            command,
            Command::Search(SearchArgs {
                query: None,
                folder: Some(PathBuf::from(".")),
                today: false,
                since_days: None,
                limit: None,
                json: false,
                ignore_case: true,
            })
        );
    }

    #[test]
    fn parses_record_command_arguments() {
        let command = parse_args([
            "hy",
            "record",
            "--cwd",
            "/tmp/demo",
            "--command",
            "cargo test",
            "--history-id",
            "42",
            "--shell",
            "zsh",
        ])
        .expect("cli should parse");

        assert_eq!(
            command,
            Command::Record(RecordArgs {
                cwd: Some(PathBuf::from("/tmp/demo")),
                command: Some(String::from("cargo test")),
                history_id: Some(String::from("42")),
                shell: Some(ShellKind::Zsh),
            })
        );
    }

    #[test]
    fn parses_init_and_install_commands() {
        let init = parse_args(["hy", "init", "bash"]).expect("init should parse");
        let install = parse_args(["hy", "install", "zsh"]).expect("install should parse");

        assert_eq!(
            init,
            Command::Init(InitArgs {
                shell: ShellKind::Bash
            })
        );
        assert_eq!(
            install,
            Command::Install(InstallArgs {
                shell: ShellKind::Zsh
            })
        );
    }
}

pub mod cli;
pub mod entry;
pub mod output;
pub mod parser;
pub mod record;
pub mod search;
pub mod shell;

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

pub fn main_entry<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    match run(args, &mut stdout, &mut stderr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            ExitCode::FAILURE
        }
    }
}

pub fn run<I, T>(args: I, stdout: &mut dyn Write, _stderr: &mut dyn Write) -> Result<(), String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let command = cli::parse_args(args)?;

    match command {
        cli::Command::Help => {
            stdout
                .write_all(output::help_text().as_bytes())
                .map_err(|err| err.to_string())?;
            Ok(())
        }
        cli::Command::Record(args) => {
            record::execute(args)?;
            Ok(())
        }
        cli::Command::Search(args) => search::execute(args, stdout),
        cli::Command::Init(args) => shell::execute_init(args, stdout),
        cli::Command::Install(args) => shell::execute_install(args, stdout),
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_without_args_prints_help() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result = run(["hy"], &mut stdout, &mut stderr);

        assert!(result.is_ok());
        let text = String::from_utf8(stdout).expect("stdout should be utf8");
        assert!(text.contains("Usage:"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_init_prints_shell_snippet() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let result = run(["hy", "init", "bash"], &mut stdout, &mut stderr);

        assert!(result.is_ok());
        let text = String::from_utf8(stdout).expect("stdout should be utf8");
        assert!(text.contains("${HY_BIN:-hy}"));
        assert!(text.contains("record --cwd"));
        assert!(stderr.is_empty());
    }
}

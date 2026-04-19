use std::process::ExitCode;

fn main() -> ExitCode {
    hy::main_entry(std::env::args_os())
}

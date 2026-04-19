use std::process::ExitCode;

fn main() -> ExitCode {
    historion::main_entry(std::env::args_os())
}

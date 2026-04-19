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

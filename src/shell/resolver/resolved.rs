use std::path::PathBuf;
use crate::shell::parser::Redirect;

#[derive(Debug,Clone)]
pub enum ResolvedCommand {
    SpecialBuiltin(CommandInvocation),
    Builtin(CommandInvocation),
    Function(CommandInvocation),
    External {
        path: PathBuf,
        invocation: CommandInvocation,
    },
}

#[derive(Debug,Clone)]
pub struct CommandInvocation {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
}


#[derive(Debug,Clone)]
pub struct ResolvedRedirections {
    pub stdin: Option<std::fs::File>,
    pub stdout: Option<std::fs::File>,
    pub stderr: Option<std::fs::File>,
}

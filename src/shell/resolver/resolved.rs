use std::path::PathBuf;

use crate::shell::parser::Redirect;

/// A simple command after name resolution, ready to execute.
#[derive(Debug, Clone)]
pub enum ResolvedCommand {
    /// Builtins whose failure affects the shell itself (e.g. `exit`).
    SpecialBuiltin(CommandInvocation),
    Builtin(CommandInvocation),
    External {
        path: PathBuf,
        invocation: CommandInvocation,
    },
}

impl ResolvedCommand {
    pub fn invocation(&self) -> &CommandInvocation {
        match self {
            Self::SpecialBuiltin(invocation) | Self::Builtin(invocation) => invocation,
            Self::External { invocation, .. } => invocation,
        }
    }

    pub fn is_external(&self) -> bool {
        matches!(self, Self::External { .. })
    }
}

#[derive(Debug, Clone)]
pub struct CommandInvocation {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
}
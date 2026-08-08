use std::env;
use std::path::{Path, PathBuf};

use super::resolved::CommandInvocation;
use super::resolved::ResolvedCommand;

use crate::shell::parser::{AST, SimpleCommand};

pub struct Resolver;

impl Resolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(&self, ast: AST) -> Result<ResolvedCommand, String> {
        match ast {
            AST::Simple(command) => self.resolve_simple(command),
        }
    }
    fn resolve_simple(&self, command: SimpleCommand) -> Result<ResolvedCommand, String> {
        
        if self.is_special_builtin(&command.command) {
            
            return Ok(ResolvedCommand::SpecialBuiltin(CommandInvocation {
                name: command.command,
                args: command.args,
                redirects: command.redirects,
            }));
        }
        if self.is_builtin(&command.command) {
            return Ok(ResolvedCommand::Builtin(CommandInvocation {
                name: command.command,
                args: command.args,
                redirects: command.redirects,
            }));
        }

        if self.is_function(&command.command) {
            return Ok(ResolvedCommand::Function(CommandInvocation {
                name: command.command,
                args: command.args,
                redirects: command.redirects,
            }));
        }
        if command.command.starts_with('/') {
            let path = PathBuf::from(&command.command);

            if path.exists() {
                return Ok(ResolvedCommand::External {
                    path,
                    invocation: CommandInvocation {
                        name: command.command,
                        args: command.args,
                        redirects: command.redirects,
                    },
                });
            }
        }
        if command.command.starts_with("./") || command.command.starts_with("../") {
            let path = PathBuf::from(&command.command);

            if path.exists() {
                
                return Ok(ResolvedCommand::External {
                    path,
                    invocation: CommandInvocation {
                        name: command.command,
                        args: command.args,
                        redirects: command.redirects,
                    },
                });
            }
        }
        if let Some(path) = self.search_path(&command.command) {
            return Ok(ResolvedCommand::External {
                path,
                invocation: CommandInvocation {
                    name: command.command,
                    args: command.args,
                    redirects: command.redirects,
                },
            });
        } else {
            return Err(format!("{}: command not found", command.command));
        }
    }
    fn is_special_builtin(&self, command: &str) -> bool {
        matches!(command,

            "break"     | 

            "continue"  |

            "return"    | 

            "exec"      | 
            
            "exit"
        )
    }

    fn is_builtin(&self, command: &str) -> bool {
        matches!(command, 

            "cd"    | 

            "pwd"   | 

            "echo"  | 

            "history"
        )
    }
    fn is_function(&self, _: &str) -> bool {
        false
    }
    fn search_path(&self, command: &str) -> Option<PathBuf> {
        let path = env::var("PATH").ok()?;

        for directory in path.split(':') {
            let candidate = Path::new(directory).join(command);

            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }
}

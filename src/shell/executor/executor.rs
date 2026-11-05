use crate::shell::resolver::redirection::{
    resolve,
    ResolvedRedirections,
};
use crate::shell::resolver::ResolvedCommand;
use crate::shell::resolver::resolved::CommandInvocation;
use crate::state::ShellState;

use std::io::{self, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        state: &mut ShellState,
        command: ResolvedCommand,
    ) -> Result<(), String> {
        // Borrow the command to get its raw redirects.
        let raw_redirects = match &command {
            ResolvedCommand::SpecialBuiltin(invocation)
            | ResolvedCommand::Builtin(invocation)
            | ResolvedCommand::Function(invocation) => &invocation.redirects,

            ResolvedCommand::External { invocation, .. } => {
                &invocation.redirects
            }
        };

        // Resolve redirects exactly once.
        let redirects = resolve(raw_redirects)?;

        // Dispatch the command with the resolved redirect handles.
        match command {
            ResolvedCommand::SpecialBuiltin(invocation) => {
                self.execute_builtin_with_redirects(redirects, || {
                    self.execute_special_builtin(state, invocation)
                })
            }

            ResolvedCommand::Builtin(invocation) => {
                self.execute_builtin_with_redirects(redirects, || {
                    self.execute_builtin(state, invocation)
                })
            }

            ResolvedCommand::Function(invocation) => {
                self.execute_function(invocation, redirects)
            }

            ResolvedCommand::External { path, invocation } => {
                self.execute_external(path, invocation, redirects)
            }
        }
    }

    fn execute_builtin_with_redirects<F>(
        &self,
        redirects: ResolvedRedirections,
        f: F,
    ) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        if redirects.is_empty() {
            return f();
        }

        // Save the shell's original stdin/stdout.
        let guard = FdRestoreGuard::new()
            .map_err(|e| format!("Failed to save file descriptors: {}", e))?;

        // Apply the resolved handles to the shell process.
        let apply_result = apply_to_shell(&redirects);

        let result = match apply_result {
            Ok(()) => f(),
            Err(error) => Err(error),
        };

        // Flush any buffered builtin output before restoring stdout.
        let _ = io::stdout().flush();

        // Restore happens in FdRestoreGuard::drop().
        drop(guard);

        result
    }

    fn execute_special_builtin(
        &self,
        _state: &mut ShellState,
        invocation: CommandInvocation,
    ) -> Result<(), String> {
        match invocation.name.as_str() {
            "exit" => {
                std::process::exit(0);
            }

            _ => Err(format!(
                "unimplemented special builtin: {}",
                invocation.name
            )),
        }
    }

    fn execute_builtin(
        &self,
        state: &mut ShellState,
        invocation: CommandInvocation,
    ) -> Result<(), String> {
        match invocation.name.as_str() {
            "echo" => {
                println!("{}", invocation.args.join(" "));
                Ok(())
            }

            "pwd" => {
                let cwd =
                    std::env::current_dir().map_err(|e| e.to_string())?;

                println!("{}", cwd.display());

                Ok(())
            }

            "cd" => self.cd(state, invocation),

            _ => Err(format!(
                "unimplemented builtin: {}",
                invocation.name
            )),
        }
    }

    fn execute_function(
        &self,
        invocation: CommandInvocation,
        _redirects: ResolvedRedirections,
    ) -> Result<(), String> {
        Err(format!(
            "function not implemented: {}",
            invocation.name
        ))
    }

    fn execute_external(
        &self,
        path: PathBuf,
        invocation: CommandInvocation,
        redirects: ResolvedRedirections,
    ) -> Result<(), String> {
        let mut command = Command::new(path);

        command.args(&invocation.args);

        // External commands use child-process stdio configuration.
        if let Some(file) = redirects.stdin {
            command.stdin(Stdio::from(file));
        }

        if let Some(file) = redirects.stdout {
            command.stdout(Stdio::from(file));
        }

        // Unconfigured streams inherit the shell's streams.
        let status = command
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!(
                "process exited with status {}",
                status
            ));
        }

        Ok(())
    }

    fn cd(
        &self,
        state: &mut ShellState,
        invocation: CommandInvocation,
    ) -> Result<(), String> {
        let raw_target = if invocation.args.is_empty() {
            "~"
        } else {
            &invocation.args[0]
        };

        let target = self.expand_cd_target(state, raw_target)?;

        let old_dir =
            std::env::current_dir().map_err(|e| e.to_string())?;

        std::env::set_current_dir(&target)
            .map_err(|e| format!("cd: {}: {}", target, e))?;

        let new_dir =
            std::env::current_dir().map_err(|e| e.to_string())?;

        state.previous_directory = Some(old_dir);
        state.cwd = new_dir.clone();

        if raw_target == "-" {
            println!("{}", new_dir.display());
        }

        Ok(())
    }

    fn expand_cd_target(
        &self,
        state: &ShellState,
        target: &str,
    ) -> Result<String, String> {
        match target {
            "-" => state
                .previous_directory
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .ok_or_else(|| "cd: OLDPWD not set".to_string()),

            "~" => std::env::var("HOME")
                .map_err(|_| "HOME not set".to_string()),

            _ if target.starts_with("~/") => {
                let home = std::env::var("HOME")
                    .map_err(|_| "HOME not set".to_string())?;

                Ok(format!("{}/{}", home, &target[2..]))
            }

            _ => Ok(target.to_string()),
        }
    }
}


/// Saves stdin/stdout and restores them when dropped.
///
/// This guard ensures that builtin redirection does not permanently
/// change the shell's own file descriptors.
struct FdRestoreGuard {
    stdin_backup: RawFd,
    stdout_backup: RawFd,
}

impl FdRestoreGuard {
    fn new() -> io::Result<Self> {
        io::stdout().flush()?;

        let stdout_backup = unsafe {
            libc::dup(libc::STDOUT_FILENO)
        };

        if stdout_backup == -1 {
            return Err(io::Error::last_os_error());
        }

        let stdin_backup = unsafe {
            libc::dup(libc::STDIN_FILENO)
        };

        if stdin_backup == -1 {
            unsafe {
                libc::close(stdout_backup);
            }

            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            stdin_backup,
            stdout_backup,
        })
    }
}

impl Drop for FdRestoreGuard {
    fn drop(&mut self) {
        let _ = io::stdout().flush();

        unsafe {
            libc::dup2(
                self.stdout_backup,
                libc::STDOUT_FILENO,
            );

            libc::dup2(
                self.stdin_backup,
                libc::STDIN_FILENO,
            );

            libc::close(self.stdout_backup);
            libc::close(self.stdin_backup);
        }
    }
}


/// Applies already-resolved file handles to the shell process.
/// Used only for in-process commands such as builtins.
fn apply_to_shell(
    redirects: &ResolvedRedirections,
) -> Result<(), String> {
    if let Some(file) = &redirects.stdin {
        let result = unsafe {
            libc::dup2(
                file.as_raw_fd(),
                libc::STDIN_FILENO,
            )
        };

        if result == -1 {
            return Err(format!(
                "Failed to redirect stdin: {}",
                io::Error::last_os_error()
            ));
        }
    }

    if let Some(file) = &redirects.stdout {
        let result = unsafe {
            libc::dup2(
                file.as_raw_fd(),
                libc::STDOUT_FILENO,
            )
        };

        if result == -1 {
            return Err(format!(
                "Failed to redirect stdout: {}",
                io::Error::last_os_error()
            ));
        }
    }

    Ok(())
}
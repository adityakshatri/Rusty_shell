use std::collections::BTreeSet;
use std::env;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use super::resolved::{CommandInvocation, ResolvedCommand};
use crate::shell::expand::ExpandedCommand;

pub const SPECIAL_BUILTINS: &[&str] = &["break", "continue", "return", "exec", "exit"];
pub const BUILTINS: &[&str] = &["cd", "pwd", "echo", "export", "unset", "history"];

/// Why a command could not be resolved. Each variant maps to a shell exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Not found in PATH (exit 127).
    NotFound(String),
    /// A path containing `/` does not exist (exit 127).
    NoSuchFile(String),
    /// Found but not executable (exit 126).
    PermissionDenied(String),
    /// A path containing `/` is a directory (exit 126).
    IsDirectory(String),
}

impl ResolveError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ResolveError::NotFound(_) | ResolveError::NoSuchFile(_) => 127,
            ResolveError::PermissionDenied(_) | ResolveError::IsDirectory(_) => 126,
        }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::NotFound(name) => write!(f, "{name}: command not found"),
            ResolveError::NoSuchFile(name) => write!(f, "{name}: No such file or directory"),
            ResolveError::PermissionDenied(name) => write!(f, "{name}: Permission denied"),
            ResolveError::IsDirectory(name) => write!(f, "{name}: Is a directory"),
        }
    }
}

enum PathSearch {
    Found(PathBuf),
    NotExecutable,
    Missing,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Resolver;

impl Resolver {
    pub fn new() -> Self {
        Self
    }

    /// Turns an expanded command into something the executor can run.
    /// The command must have at least one word.
    pub fn resolve(&self, command: &ExpandedCommand) -> Result<ResolvedCommand, ResolveError> {
        let Some((name, args)) = command.words.split_first() else {
            return Err(ResolveError::NotFound(String::new()));
        };
        let name = name.as_str();

        let invocation = || CommandInvocation {
            name: name.to_string(),
            args: args.to_vec(),
            redirects: command.redirects.clone(),
        };

        if SPECIAL_BUILTINS.contains(&name) {
            return Ok(ResolvedCommand::SpecialBuiltin(invocation()));
        }
        if BUILTINS.contains(&name) {
            return Ok(ResolvedCommand::Builtin(invocation()));
        }

        if name.contains('/') {
            return self.resolve_path(name, invocation());
        }

        match self.search_path(name) {
            PathSearch::Found(path) => Ok(ResolvedCommand::External {
                path,
                invocation: invocation(),
            }),
            PathSearch::NotExecutable => Err(ResolveError::PermissionDenied(name.to_string())),
            PathSearch::Missing => Err(ResolveError::NotFound(name.to_string())),
        }
    }

    /// Names offered by tab completion: builtins plus every executable on PATH.
    pub fn command_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = SPECIAL_BUILTINS
            .iter()
            .chain(BUILTINS.iter())
            .map(|name| name.to_string())
            .collect();

        if let Some(path_var) = env::var_os("PATH") {
            for dir in env::split_paths(&path_var) {
                let Ok(entries) = fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if is_executable(&path) {
                        names.insert(entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }

        names
    }

    fn resolve_path(&self, name: &str, invocation: CommandInvocation) -> Result<ResolvedCommand, ResolveError> {
        let path = PathBuf::from(name);

        let metadata = fs::metadata(&path).map_err(|_| ResolveError::NoSuchFile(name.to_string()))?;

        if metadata.is_dir() {
            return Err(ResolveError::IsDirectory(name.to_string()));
        }
        if !is_executable(&path) {
            return Err(ResolveError::PermissionDenied(name.to_string()));
        }

        Ok(ResolvedCommand::External { path, invocation })
    }

    /// Walks PATH in order. A non-executable match does not end the search.
    /// If nothing executable turns up, that is reported as a permission error.
    fn search_path(&self, name: &str) -> PathSearch {
        let Some(path_var) = env::var_os("PATH") else {
            return PathSearch::Missing;
        };

        let mut saw_non_executable = false;

        for dir in env::split_paths(&path_var) {
            let candidate = if dir.as_os_str().is_empty() {
                Path::new(".").join(name)
            } else {
                dir.join(name)
            };

            if !candidate.is_file() {
                continue;
            }
            if is_executable(&candidate) {
                return PathSearch::Found(candidate);
            }
            saw_non_executable = true;
        }

        if saw_non_executable {
            PathSearch::NotExecutable
        } else {
            PathSearch::Missing
        }
    }
}

/// True if `path` is a regular file the current user can execute.
pub fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };

    // access(2) checks the real permissions for this user, not just the mode bits.
    unsafe { libc::access(c_path.as_ptr(), libc::X_OK) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_executable_file_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("script.sh");
        std::fs::write(&file, "echo hi").unwrap();

        let path = file.to_string_lossy().into_owned();
        let command = ExpandedCommand {
            words: vec![path.clone()],
            assignments: vec![],
            redirects: vec![],
        };

        let err = Resolver::new().resolve(&command).unwrap_err();
        assert_eq!(err, ResolveError::PermissionDenied(path));
    }

    #[test]
    fn builtins_resolve_without_path_lookup() {
        let command = ExpandedCommand {
            words: vec!["echo".to_string(), "hi".to_string()],
            assignments: vec![],
            redirects: vec![],
        };
        assert!(matches!(
            Resolver::new().resolve(&command),
            Ok(ResolvedCommand::Builtin(_))
        ));
    }
}
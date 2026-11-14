use std::fs::File;
use std::io::{self, PipeReader, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::{env, thread};

use crate::shell::expand::expand_command;
use crate::shell::parser::{AndOr, Connector, Pipeline, Separator, AST};
use crate::shell::resolver::redirection::{resolve as resolve_redirects, ResolvedRedirections};
use crate::shell::resolver::resolved::CommandInvocation;
use crate::shell::resolver::{ResolvedCommand, Resolver};
use crate::state::{is_valid_name, ShellState};

pub struct Executor;

/// One pipeline stage after expansion and resolution.
struct Stage {
    command: ResolvedCommand,
    /// `NAME=value` prefixes. These only reach external commands.
    assignments: Vec<(String, String)>,
}

impl Executor {
    pub fn new() -> Self {
        Self
    }

    /// Runs every item of a parsed line and returns the status of the last one.
    pub fn execute(&self, state: &mut ShellState, resolver: &Resolver, ast: AST) -> i32 {
        let mut status = 0;

        for item in ast.items {
            let background = item.separator == Separator::Background;

            status = self.run_and_or(state, resolver, &item.and_or, background);
            if background {
                status = 0; // like bash, `&` always reports success
            }

            state.last_status = status;

            if state.exit_requested.is_some() {
                break;
            }
        }

        status
    }

    /// Collects finished background children so they don't linger as zombies.
    pub fn reap_background(&self, state: &mut ShellState) {
        state
            .background
            .retain_mut(|child| matches!(child.try_wait(), Ok(None)));
    }

    fn run_and_or(&self, state: &mut ShellState, resolver: &Resolver, and_or: &AndOr, background: bool) -> i32 {
        // Only a lone pipeline can go to the background. Chains run in the foreground.
        let first_background = background && and_or.rest.is_empty();
        let mut status = self.run_pipeline(state, resolver, &and_or.first, first_background);

        for (connector, pipeline) in &and_or.rest {
            let should_run = match connector {
                Connector::And => status == 0,
                Connector::Or => status != 0,
            };

            if should_run {
                status = self.run_pipeline(state, resolver, pipeline, false);
            }

            if state.exit_requested.is_some() {
                break;
            }
        }

        status
    }

    fn run_pipeline(&self, state: &mut ShellState, resolver: &Resolver, pipeline: &Pipeline, background: bool) -> i32 {
        // Expand and resolve every stage before starting any of them.
        let mut stages = Vec::with_capacity(pipeline.commands.len());
        for simple in &pipeline.commands {
            let expanded = match expand_command(simple, state) {
                Ok(expanded) => expanded,
                Err(message) => {
                    eprintln!("{message}");
                    return 1;
                }
            };

            if expanded.words.is_empty() {
                // `NAME=value` alone sets shell variables in the current shell.
                for (name, value) in expanded.assignments {
                    state.set_var(name, value);
                }
                continue;
            }

            match resolver.resolve(&expanded) {
                Ok(command) => stages.push(Stage {
                    command,
                    assignments: expanded.assignments,
                }),
                Err(err) => {
                    eprintln!("{err}");
                    return err.exit_code();
                }
            }
        }

        if stages.is_empty() {
            return 0;
        }

        // Only pipelines of external programs can run in the background.
        // Builtins would need a subshell, so those run in the foreground.
        let background = background && stages.iter().all(|stage| stage.command.is_external());

        let stage_count = stages.len();
        let base_env = state.exported_env();

        let mut children: Vec<Child> = Vec::new();
        let mut last_child: Option<usize> = None;
        let mut last_status = 0;

        // Output of the previous stage, waiting for this one.
        let mut next_stdin: Option<PipeReader> = None; // from an external stage
        let mut next_bytes: Option<Vec<u8>> = None; // from a builtin stage

        for (index, stage) in stages.into_iter().enumerate() {
            let is_last = index + 1 == stage_count;
            let Stage { command, assignments } = stage;

            let ResolvedRedirections {
                stdin: file_stdin,
                stdout: mut file_stdout,
            } = match resolve_redirects(&command.invocation().redirects) {
                Ok(redirects) => redirects,
                Err(message) => {
                    eprintln!("{message}");
                    last_status = 1;
                    break;
                }
            };

            // An explicit `<` overrides the pipe from the previous stage.
            let stdin = match take_stdin(file_stdin, &mut next_stdin, &mut next_bytes) {
                Ok(stdin) => stdin,
                Err(message) => {
                    eprintln!("{message}");
                    last_status = 1;
                    break;
                }
            };

            match command {
                ResolvedCommand::SpecialBuiltin(invocation) | ResolvedCommand::Builtin(invocation) => {
                    let mut captured: Vec<u8> = Vec::new();

                    let status = match file_stdout.as_mut() {
                        Some(file) => self.run_builtin(state, &invocation, file),
                        None if !is_last => self.run_builtin(state, &invocation, &mut captured),
                        None => self.run_builtin(state, &invocation, &mut io::stdout().lock()),
                    };

                    if !is_last && file_stdout.is_none() {
                        next_bytes = Some(captured);
                    }
                    if is_last {
                        last_status = status;
                    }
                }

                ResolvedCommand::External { path, invocation } => {
                    let mut process = Command::new(path);
                    process.arg0(&invocation.name).args(&invocation.args);

                    // The child gets exactly the exported variables plus its own prefixes.
                    process
                        .env_clear()
                        .envs(base_env.iter().cloned())
                        .envs(assignments);

                    if let Some(stdin) = stdin {
                        process.stdin(stdin);
                    }

                    if let Some(file) = file_stdout.take() {
                        process.stdout(Stdio::from(file));
                    } else if !is_last {
                        match io::pipe() {
                            Ok((reader, writer)) => {
                                process.stdout(Stdio::from(writer));
                                next_stdin = Some(reader);
                            }
                            Err(err) => {
                                eprintln!("pipe: {err}");
                                last_status = 1;
                                break;
                            }
                        }
                    }

                    match process.spawn() {
                        Ok(child) => {
                            if is_last {
                                last_child = Some(children.len());
                            }
                            children.push(child);
                        }
                        Err(err) => {
                            eprintln!("{}: {}", invocation.name, describe_io_error(&err));
                            if is_last {
                                last_status = spawn_failure_code(&err);
                            }
                        }
                    }
                }
            }
        }

        if background {
            state.background.extend(children);
            return 0;
        }

        let mut codes = Vec::with_capacity(children.len());
        for mut child in children {
            codes.push(child.wait().map(status_code).unwrap_or(1));
        }

        if let Some(index) = last_child {
            last_status = codes[index];
        }

        last_status
    }

    fn run_builtin(&self, state: &mut ShellState, invocation: &CommandInvocation, out: &mut dyn Write) -> i32 {
        if invocation.name == "exit" {
            return self.exit(state, invocation);
        }

        let result = match invocation.name.as_str() {
            "echo" => writeln!(out, "{}", invocation.args.join(" ")).map_err(|e| e.to_string()),

            "pwd" => env::current_dir()
                .map_err(|e| e.to_string())
                .and_then(|cwd| writeln!(out, "{}", cwd.display()).map_err(|e| e.to_string())),

            "cd" => self.cd(state, invocation, out),
            "export" => self.export(state, invocation),
            "unset" => self.unset(state, invocation),

            other => Err(format!("{other}: builtin not implemented")),
        };

        match result {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("{message}");
                1
            }
        }
    }

    /// `exit [N]`. With no argument, the status of the last command is used.
    fn exit(&self, state: &mut ShellState, invocation: &CommandInvocation) -> i32 {
        let code = match invocation.args.first() {
            None => state.last_status,
            Some(arg) => match arg.parse::<i32>() {
                Ok(n) => n & 0xff,
                Err(_) => {
                    eprintln!("exit: {arg}: numeric argument required");
                    2
                }
            },
        };

        state.exit_requested = Some(code);
        code
    }

    /// `export NAME[=value]...`
    fn export(&self, state: &mut ShellState, invocation: &CommandInvocation) -> Result<(), String> {
        for arg in &invocation.args {
            match arg.split_once('=') {
                Some((name, value)) => {
                    check_name("export", name)?;
                    state.export_var(name, Some(value.to_string()));
                }
                None => {
                    check_name("export", arg)?;
                    state.export_var(arg, None);
                }
            }
        }
        Ok(())
    }

    /// `unset NAME...`
    fn unset(&self, state: &mut ShellState, invocation: &CommandInvocation) -> Result<(), String> {
        for arg in &invocation.args {
            check_name("unset", arg)?;
            state.unset_var(arg);
        }
        Ok(())
    }

    fn cd(&self, state: &mut ShellState, invocation: &CommandInvocation, out: &mut dyn Write) -> Result<(), String> {
        let raw_target = invocation.args.first().map(String::as_str).unwrap_or("~");

        let target = self.expand_cd_target(state, raw_target)?;

        let old_dir = env::current_dir().map_err(|e| e.to_string())?;

        env::set_current_dir(&target).map_err(|e| format!("cd: {}: {}", target, e))?;

        let new_dir = env::current_dir().map_err(|e| e.to_string())?;

        state.set_var("OLDPWD", old_dir.display().to_string());
        state.set_var("PWD", new_dir.display().to_string());
        state.previous_directory = Some(old_dir);
        state.cwd = new_dir.clone();

        if raw_target == "-" {
            writeln!(out, "{}", new_dir.display()).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn expand_cd_target(&self, state: &ShellState, target: &str) -> Result<String, String> {
        match target {
            "-" => state
                .previous_directory
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .ok_or_else(|| "cd: OLDPWD not set".to_string()),

            "~" => home(state),

            _ if target.starts_with("~/") => Ok(format!("{}/{}", home(state)?, &target[2..])),

            _ => Ok(target.to_string()),
        }
    }
}

fn home(state: &ShellState) -> Result<String, String> {
    state
        .get_var("HOME")
        .map(str::to_string)
        .ok_or_else(|| "HOME not set".to_string())
}

fn check_name(builtin: &str, name: &str) -> Result<(), String> {
    if is_valid_name(name) {
        Ok(())
    } else {
        Err(format!("{builtin}: `{name}': not a valid identifier"))
    }
}

/// Decides what a stage reads from. An explicit `<` file wins over a pipe.
fn take_stdin(
    file: Option<File>,
    next_stdin: &mut Option<PipeReader>,
    next_bytes: &mut Option<Vec<u8>>,
) -> Result<Option<Stdio>, String> {
    if let Some(file) = file {
        return Ok(Some(Stdio::from(file)));
    }

    if let Some(reader) = next_stdin.take() {
        return Ok(Some(Stdio::from(reader)));
    }

    if let Some(bytes) = next_bytes.take() {
        // Feed captured builtin output through a pipe. A thread does the write,
        // so a large buffer can't block the shell before the child starts reading.
        let (reader, mut writer) = io::pipe().map_err(|e| format!("pipe: {e}"))?;
        thread::spawn(move || {
            let _ = writer.write_all(&bytes);
        });
        return Ok(Some(Stdio::from(reader)));
    }

    Ok(None)
}

fn status_code(status: ExitStatus) -> i32 {
    status
        .code()
        .or_else(|| status.signal().map(|sig| 128 + sig))
        .unwrap_or(1)
}

fn spawn_failure_code(err: &io::Error) -> i32 {
    match err.kind() {
        io::ErrorKind::NotFound => 127,
        _ => 126,
    }
}

fn describe_io_error(err: &io::Error) -> String {
    match err.kind() {
        io::ErrorKind::NotFound => "No such file or directory".to_string(),
        io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
        _ => err.to_string(),
    }
}
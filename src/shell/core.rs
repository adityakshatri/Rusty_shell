use std::sync::{Arc, Mutex};

use crate::frontend::ShellCompleter;
use crate::shell::executor::Executor;
use crate::shell::lexer::Lexer;
use crate::shell::parser::Parser;
use crate::shell::resolver::Resolver;
use crate::state::ShellState;

use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

pub struct Shell {
    pub state: ShellState,
    pub resolver: Resolver,
    pub executor: Executor,
    /// Variable names shared with the completer. Refreshed before each prompt.
    completion_vars: Arc<Mutex<Vec<String>>>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            state: ShellState::new(),
            resolver: Resolver::new(),
            executor: Executor::new(),
            completion_vars: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // Parses and runs one line of input, with no line editor involved.
    // Returns `Some(code)` when the shell should exit.
    pub fn run_line(&mut self, line: &str) -> Option<i32> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        if line == "quit" {
            return Some(self.state.last_status);
        }

        let tokens = Lexer::new(line).lex();

        let ast = match Parser::new(tokens).parse() {
            Ok(ast) => ast,
            Err(err) => {
                eprintln!("{err}");
                self.state.last_status = 2;
                return None;
            }
        };

        self.executor.execute(&mut self.state, &self.resolver, ast);
        self.state.exit_requested
    }

    /// Interactive loop with line editing and completion. Returns the process exit code.
    pub fn run(&mut self) -> i32 {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let mut editor = match Editor::<ShellCompleter, _>::with_config(config) {
            Ok(editor) => editor,
            Err(error) => {
                eprintln!("failed to initialize line editor: {error}");
                return 1;
            }
        };
        editor.set_helper(Some(ShellCompleter::new(
            self.resolver.clone(),
            Arc::clone(&self.completion_vars),
        )));

        loop {
            self.executor.reap_background(&mut self.state);
            self.refresh_completion_vars();

            let input = match editor.readline("$ ") {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(error) => {
                    eprintln!("readline error: {error}");
                    break;
                }
            };

            if let Some(code) = self.run_line(&input) {
                return code;
            }
        }

        self.state.last_status
    }

    fn refresh_completion_vars(&self) {
        if let Ok(mut names) = self.completion_vars.lock() {
            *names = self.state.var_names();
        }
    }
}
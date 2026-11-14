// Core shell loop and state management.
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
}

impl Shell {
    pub fn new() -> Self {
        Self {
            state: ShellState::new(),
            resolver: Resolver::new(),
            executor: Executor::new(),
        }
    }

    /// Runs the interactive loop. Returns the exit status for the process.
    pub fn run(&mut self) -> i32 {
        let config = Config::builder()
    .       completion_type(CompletionType::List)
    .       build();

        let mut editor = match Editor::<ShellCompleter, _>::with_config(config) {
            Ok(editor) => editor,
            Err(error) => {
                eprintln!("failed to initialize line editor: {error}");
                return 1;
            }
        };
        editor.set_helper(Some(ShellCompleter::new(self.resolver.clone())));

        loop {
            self.executor.reap_background(&mut self.state);

            let input = match editor.readline("$ ") {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(error) => {
                    eprintln!("readline error: {error}");
                    break;
                }
            };

            let line = input.trim();
            if line.is_empty() {
                continue;
            }

            let tokens = Lexer::new(line).lex();

            let ast = match Parser::new(tokens).parse() {
                Ok(ast) => ast,
                Err(err) => {
                    eprintln!("{err}");
                    self.state.last_status = 2;
                    continue;
                }
            };

            self.executor.execute(&mut self.state, &self.resolver, ast);

            if let Some(code) = self.state.exit_requested {
                return code;
            }
        }

        self.state.last_status
    }
}
//shell.rs
use crate::shell::executor::Executor;
use crate::shell::lexer::Lexer;
use crate::shell::parser::Parser;
use crate::shell::resolver::Resolver;
use crate::state::ShellState;

use crate::frontend::ShellCompleter;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};

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
    pub fn run(&mut self) {
        let config = Config::default();

        let mut editor =
         match Editor::<ShellCompleter, _>::with_config(config) {
            Ok(editor) => editor,
            Err(error) => {
                eprintln!("failed to initialize line editor: {error}");
                return;
            }
        };
        editor.set_helper(Some(ShellCompleter::new()));
        loop {
            let input = match editor.readline("$ ") {
                Ok(line) => line,

                Err(ReadlineError::Interrupted) => {
                    continue;
                }

                Err(ReadlineError::Eof) => {
                    break;
                }

                Err(error) => {
                    eprintln!("readline error: {error}");
                    break;
                }
            };

            if input.is_empty() {
                continue;
            }

            if (input.to_lowercase() == "exit") || (input.to_lowercase() == "quit") {
                break;
            }

            let lexer = Lexer::new(&input);

            let tokens = lexer.lex();

            println!("{:#?}", tokens);

            let mut parser = Parser::new(tokens);

            let ast = match parser.parse() {
                Ok(ast) => ast,

                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            };

            println!("{:#?}", ast);
            let resolver = Resolver::new();

            let resolved = match resolver.resolve(ast) {
                Ok(command) => command,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            println!("{resolved:#?}");
            let executor = Executor::new();

            if let Err(err) = executor.execute(&mut self.state, resolved) {
                eprintln!("{err}");
            }
        }
    }
}

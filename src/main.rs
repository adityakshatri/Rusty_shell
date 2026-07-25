mod shell;
mod lexer;
mod parser;
mod resolver;
mod executor;
mod state;

use shell::Shell;

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
//entry point of the shell program
mod frontend;
mod shell;
mod state;

use crate::shell::core::Shell;

fn main() {
    let mut shell = Shell::new();
    let code = shell.run();
    std::process::exit(code);
}
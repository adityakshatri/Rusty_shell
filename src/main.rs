mod shell;
mod state;
mod frontend;

use crate::shell::core::Shell;

fn main() {
    let mut shell = Shell::new();
    shell.run();
}
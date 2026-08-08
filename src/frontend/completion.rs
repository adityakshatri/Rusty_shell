use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct ShellCompleter;

impl ShellCompleter {
    pub fn new() -> Self {
        Self
    }

    fn builtin_names() -> &'static [&'static str] {
        &["cd", "pwd", "echo", "exit", "export", "help", "history"]
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Only inspect the part of the line before the cursor.
        let before_cursor = &line[..pos];

        // Find the beginning of the current word.
        let start = before_cursor
            .rfind(char::is_whitespace)
            .map(|index| index + 1)
            .unwrap_or(0);

        let prefix = &before_cursor[start..];

        // For now, only complete the first word.
        if start != 0 {
            return Ok((start, Vec::new()));
        }

        let candidates = Self::builtin_names()
            .iter()
            .filter(|name| name.starts_with(prefix))
            .map(|name| Pair {
                display: (*name).to_string(),
                replacement: format!("{} ", name),
            })
            .collect();

        Ok((start, candidates))
    }
}

//TODOs
// Rustyline's Helper requires Completer + Hinter + Highlighter + Validator.
impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}

impl Helper for ShellCompleter {}

use std::env;
use std::fs;
use std::path::PathBuf;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::shell::resolver::Resolver;

/// Characters that end a word on the command line.
const WORD_DELIMITERS: &[char] = &[' ', '\t', '|', '&', ';', '<', '>', '(', ')'];

/// Characters that get a backslash when they appear in a completed filename.
const ESCAPED: &[char] = &[
    ' ', '\t', '\'', '"', '\\', '|', '&', ';', '<', '>', '(', ')', '$',
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathFilter {
    Any,
    DirectoriesOnly,
}

pub struct ShellCompleter {
    resolver: Resolver,
}

impl ShellCompleter {
    pub fn new(resolver: Resolver) -> Self {
        Self { resolver }
    }

    /// Builtins and executables on PATH, filtered by prefix.
    fn command_candidates(&self, prefix: &str) -> Vec<Pair> {
        self.resolver
            .command_names()
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .map(|name| Pair {
                display: name.clone(),
                replacement: format!("{name} "),
            })
            .collect()
    }

    /// Files and directories relative to the typed directory part.
    fn path_candidates(word: &str, filter: PathFilter) -> Vec<Pair> {
        if word == "~" {
            return vec![Pair {
                display: "~/".to_string(),
                replacement: "~/".to_string(),
            }];
        }

        // Split "some/dir/par" into "some/dir/" and "par".
        let (typed_dir, partial) = match word.rfind('/') {
            Some(index) => word.split_at(index + 1),
            None => ("", word),
        };

        let search_dir = if typed_dir.is_empty() {
            PathBuf::from(".")
        } else {
            expand_tilde(typed_dir)
        };

        let Ok(entries) = fs::read_dir(&search_dir) else {
            return Vec::new();
        };

        let show_hidden = partial.starts_with('.');

        let mut pairs: Vec<Pair> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();

                if !name.starts_with(partial) {
                    return None;
                }
                if name.starts_with('.') && !show_hidden {
                    return None;
                }

                let is_dir = entry.path().is_dir();
                if filter == PathFilter::DirectoriesOnly && !is_dir {
                    return None;
                }

                let (display, suffix) = if is_dir {
                    (format!("{name}/"), "/")
                } else {
                    (name.clone(), " ")
                };

                Some(Pair {
                    display,
                    replacement: format!("{typed_dir}{}{suffix}", escape(&name)),
                })
            })
            .collect();

        pairs.sort_by(|a, b| a.display.cmp(&b.display));
        pairs
    }

    /// Hook for `$VAR` completion. Returns nothing until shell variables exist.
    fn variable_candidates(&self, _prefix: &str) -> Vec<Pair> {
        // TODO: once ShellState tracks variables, list names that start with `prefix`
        // and return replacements that keep the leading `$`.
        Vec::new()
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
        let before_cursor = &line[..pos];

        let start = before_cursor
            .rfind(|c: char| WORD_DELIMITERS.contains(&c))
            .map(|index| index + 1)
            .unwrap_or(0);

        let word = &before_cursor[start..];
        let preceding = &before_cursor[..start];

        // Only the current command segment matters: text after the last `|`, `&`, `;` or `(`.
        let segment = preceding
            .rsplit(['|', '&', ';', '('])
            .next()
            .unwrap_or("");
        let at_command_start = segment.trim().is_empty();
        let command = segment.split_whitespace().next();

        let candidates = if word.starts_with('$') {
            self.variable_candidates(&word[1..])
        } else if at_command_start {
            self.command_candidates(word)
        } else if command == Some("cd") {
            Self::path_candidates(word, PathFilter::DirectoriesOnly)
        } else {
            Self::path_candidates(word, PathFilter::Any)
        };

        Ok((start, candidates))
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn escape(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if ESCAPED.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

// Rustyline's Helper requires Completer + Hinter + Highlighter + Validator.
impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}

impl Helper for ShellCompleter {}


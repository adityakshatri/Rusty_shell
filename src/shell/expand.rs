// expansion of words and commands. This is separate from parsing because it needs to be done after parsing, and it also needs access to the shell state (environment variables, etc.).
use std::process;

use crate::shell::lexer::{Quoting, Word};
use crate::shell::parser::{RedirectMode, SimpleCommand};
use crate::state::{is_valid_name, ShellState};

#[derive(Debug, Clone)]
pub struct ExpandedRedirect {
    pub mode: RedirectMode,
    pub target: String,
}

/// A command after expansion. Everything here is plain strings.
#[derive(Debug, Clone, Default)]
pub struct ExpandedCommand {
    /// argv. Empty for an assignment-only command.
    pub words: Vec<String>,
    pub assignments: Vec<(String, String)>,
    pub redirects: Vec<ExpandedRedirect>,
}

pub fn expand_command(command: &SimpleCommand, state: &ShellState) -> Result<ExpandedCommand, String> {
    let mut words = Vec::new();
    for word in &command.words {
        words.extend(expand_word(word, state)?);
    }

    let mut assignments = Vec::new();
    for assignment in &command.assignments {
        let value = expand_to_string(&assignment.value, state)?;
        assignments.push((assignment.name.clone(), value));
    }

    let mut redirects = Vec::new();
    for redirect in &command.redirects {
        let mut fields = expand_word(&redirect.target, state)?;
        if fields.len() != 1 {
            return Err(format!("{}: ambiguous redirect", redirect.target.raw_text()));
        }
        redirects.push(ExpandedRedirect {
            mode: redirect.mode.clone(),
            target: fields.remove(0),
        });
    }

    Ok(ExpandedCommand {
        words,
        assignments,
        redirects,
    })
}

/// Expands a word into fields. Unquoted expansions are split on whitespace,
/// and a quoted empty string still produces one empty field.
pub fn expand_word(word: &Word, state: &ShellState) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut field_open = false;

    for (index, part) in word.parts.iter().enumerate() {
        let text = expand_part(index, part.quoting, &part.text, state)?;

        match part.quoting {
            Quoting::Unquoted => {
                for c in text.chars() {
                    if c.is_whitespace() {
                        if field_open {
                            fields.push(std::mem::take(&mut current));
                            field_open = false;
                        }
                    } else {
                        current.push(c);
                        field_open = true;
                    }
                }
            }
            Quoting::Double | Quoting::Single => {
                current.push_str(&text);
                field_open = true;
            }
        }
    }

    if field_open {
        fields.push(current);
    }

    if fields.is_empty() && word.parts.is_empty() {
        fields.push(String::new());
    }

    Ok(fields)
}

/// Expands a word without splitting. Used for assignment values.
pub fn expand_to_string(word: &Word, state: &ShellState) -> Result<String, String> {
    let mut out = String::new();
    for (index, part) in word.parts.iter().enumerate() {
        out.push_str(&expand_part(index, part.quoting, &part.text, state)?);
    }
    Ok(out)
}

fn expand_part(index: usize, quoting: Quoting, text: &str, state: &ShellState) -> Result<String, String> {
    match quoting {
        Quoting::Single => Ok(text.to_string()),
        Quoting::Double => expand_dollars(text, state),
        Quoting::Unquoted => {
            // Tilde only expands at the start of the word.
            let text = if index == 0 {
                expand_tilde(text, state)
            } else {
                text.to_string()
            };
            expand_dollars(&text, state)
        }
    }
}

fn expand_tilde(text: &str, state: &ShellState) -> String {
    if text == "~" || text.starts_with("~/") {
        if let Some(home) = state.get_var("HOME") {
            return format!("{home}{}", &text[1..]);
        }
    }
    text.to_string()
}

/// Expands `$NAME`, `${NAME}`, `$?`, and `$$`. A lone `$` stays a literal `$`.
fn expand_dollars(text: &str, state: &ShellState) -> Result<String, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c != '$' {
            out.push(c);
            i += 1;
            continue;
        }

        i += 1;
        match chars.get(i).copied() {
            None => out.push('$'),

            Some('?') => {
                out.push_str(&state.last_status.to_string());
                i += 1;
            }

            Some('$') => {
                out.push_str(&process::id().to_string());
                i += 1;
            }

            Some('{') => {
                let close = chars[i + 1..]
                    .iter()
                    .position(|&ch| ch == '}')
                    .ok_or_else(|| "bad substitution: missing `}`".to_string())?;
                let name: String = chars[i + 1..i + 1 + close].iter().collect();
                if !is_valid_name(&name) {
                    return Err(format!("${{{name}}}: bad substitution"));
                }
                out.push_str(state.get_var(&name).unwrap_or(""));
                i = i + 1 + close + 1;
            }

            Some(first) if first.is_ascii_alphabetic() || first == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                out.push_str(state.get_var(&name).unwrap_or(""));
            }

            // Not a variable reference. Keep the `$` and handle the next char normally.
            Some(_) => out.push('$'),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::lexer::{Lexer, Token};

    fn words(input: &str) -> Vec<Word> {
        Lexer::new(input)
            .lex()
            .into_iter()
            .filter_map(|token| match token {
                Token::Word(word) => Some(word),
                _ => None,
            })
            .collect()
    }

    fn expand(input: &str, state: &ShellState) -> Vec<String> {
        expand_word(&words(input)[0], state).unwrap()
    }

    fn state_with(name: &str, value: &str) -> ShellState {
        let mut state = ShellState::new();
        state.set_var(name, value.to_string());
        state
    }

    #[test]
    fn unquoted_expansion_is_split() {
        let state = state_with("RUSTY_T1", "a b");
        assert_eq!(expand("$RUSTY_T1", &state), ["a", "b"]);
    }

    #[test]
    fn double_quotes_keep_spaces() {
        let state = state_with("RUSTY_T2", "a b");
        assert_eq!(expand("\"$RUSTY_T2\"", &state), ["a b"]);
    }

    #[test]
    fn single_quotes_are_literal() {
        let state = state_with("RUSTY_T3", "a b");
        assert_eq!(expand("'$RUSTY_T3'", &state), ["$RUSTY_T3"]);
    }

    #[test]
    fn unset_unquoted_variable_disappears() {
        let state = ShellState::new();
        assert!(expand("$RUSTY_NOT_SET_XYZ", &state).is_empty());
    }

    #[test]
    fn empty_quotes_make_an_argument() {
        let state = ShellState::new();
        assert_eq!(expand("\"\"", &state), [""]);
    }

    #[test]
    fn last_status_expands() {
        let mut state = ShellState::new();
        state.last_status = 7;
        assert_eq!(expand("$?", &state), ["7"]);
    }
}
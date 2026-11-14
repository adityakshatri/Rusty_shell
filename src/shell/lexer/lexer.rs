use super::scanner::Scanner;
use super::state::LexerState;
use super::token::{Quoting, Token, Word, WordPart};

pub struct Lexer<'a> {
    scanner: Scanner<'a>,
    state: LexerState,
    tokens: Vec<Token>,
    parts: Vec<WordPart>,
    word_started: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            scanner: Scanner::new(input),
            state: LexerState::Normal,
            tokens: Vec::new(),
            parts: Vec::new(),
            word_started: false,
        }
    }

    pub fn lex(mut self) -> Vec<Token> {
        while !self.scanner.is_at_end() {
            match self.state {
                LexerState::Normal => self.lex_normal(),
                LexerState::DoubleQuote => self.lex_double_quote(),
                LexerState::SingleQuote => self.lex_single_quote(),
                LexerState::Escape => self.lex_escape(),
            }
        }

        self.emit_word();
        self.tokens.push(Token::EOF);
        self.tokens
    }

    fn lex_normal(&mut self) {
        let Some(current) = self.scanner.current() else {
            return;
        };

        match current {
            c if c.is_whitespace() => {
                self.emit_word();
                self.scanner.advance();
            }

            '\'' => {
                self.open_quote(Quoting::Single);
                self.state = LexerState::SingleQuote;
                self.scanner.advance();
            }

            '"' => {
                self.open_quote(Quoting::Double);
                self.state = LexerState::DoubleQuote;
                self.scanner.advance();
            }

            '\\' => {
                self.word_started = true;
                self.state = LexerState::Escape;
                self.scanner.advance();
            }

            '|' => {
                if self.scanner.peek() == Some('|') {
                    self.push_operator(Token::OrIf, 2);
                } else {
                    self.push_operator(Token::Pipe, 1);
                }
            }

            '&' => {
                if self.scanner.peek() == Some('&') {
                    self.push_operator(Token::AndIf, 2);
                } else {
                    self.push_operator(Token::Background, 1);
                }
            }

            '>' => {
                if self.scanner.peek() == Some('>') {
                    self.push_operator(Token::RedirectAppend, 2);
                } else {
                    self.push_operator(Token::RedirectOut, 1);
                }
            }

            '<' => self.push_operator(Token::RedirectIn, 1),
            ';' => self.push_operator(Token::Semicolon, 1),
            '(' => self.push_operator(Token::LeftParen, 1),
            ')' => self.push_operator(Token::RightParen, 1),

            _ => {
                self.word_started = true;
                self.push_char(Quoting::Unquoted, current);
                self.scanner.advance();
            }
        }
    }

    /// Ends the current word, emits an operator, and consumes its characters.
    fn push_operator(&mut self, token: Token, width: usize) {
        self.emit_word();
        self.tokens.push(token);
        for _ in 0..width {
            self.scanner.advance();
        }
    }

    /// Appends a character, merging it into the last part if the quoting matches.
    fn push_char(&mut self, quoting: Quoting, c: char) {
        match self.parts.last_mut() {
            Some(part) if part.quoting == quoting => part.text.push(c),
            _ => self.parts.push(WordPart {
                quoting,
                text: c.to_string(),
            }),
        }
    }

    /// Starts a quoted section. An empty part is added right away so `""` still makes an argument.
    fn open_quote(&mut self, quoting: Quoting) {
        self.word_started = true;
        self.parts.push(WordPart {
            quoting,
            text: String::new(),
        });
    }

    fn lex_single_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            self.scanner.advance();
            if c == '\'' {
                self.state = LexerState::Normal;
                return;
            }
            self.push_char(Quoting::Single, c);
        }
    }

    fn lex_double_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            self.scanner.advance();
            if c == '"' {
                self.state = LexerState::Normal;
                return;
            }
            self.push_char(Quoting::Double, c);
        }
    }

    /// The character after a backslash is taken literally.
    fn lex_escape(&mut self) {
        if let Some(c) = self.scanner.current() {
            self.push_char(Quoting::Single, c);
            self.scanner.advance();
        }
        self.state = LexerState::Normal;
    }

    fn emit_word(&mut self) {
        if self.word_started {
            let parts = std::mem::take(&mut self.parts);
            self.tokens.push(Token::Word(Word { parts }));
            self.word_started = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        Lexer::new(input).lex()
    }

    #[test]
    fn operators_are_split() {
        let tokens = lex("a&&b||c|d&e;f");
        assert!(tokens.contains(&Token::AndIf));
        assert!(tokens.contains(&Token::OrIf));
        assert!(tokens.contains(&Token::Pipe));
        assert!(tokens.contains(&Token::Background));
        assert!(tokens.contains(&Token::Semicolon));
    }

    #[test]
    fn single_quotes_are_marked_literal() {
        let tokens = lex("'$X'");
        let Token::Word(word) = &tokens[0] else {
            panic!("expected a word");
        };
        assert_eq!(word.parts[0].quoting, Quoting::Single);
        assert_eq!(word.parts[0].text, "$X");
    }

    #[test]
    fn empty_quotes_make_a_word() {
        let tokens = lex("\"\"");
        assert!(matches!(&tokens[0], Token::Word(w) if w.parts.len() == 1));
    }
}
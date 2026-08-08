use super::scanner::Scanner;
use super::state::LexerState;
use super::token::Token;
// lexer state
pub struct Lexer<'a> {
    scanner: Scanner<'a>,
    state: LexerState,
    tokens: Vec<Token>,
    current_word: String,
}

// #[derive(Debug)]
// pub enum LexError {
//     UnterminatedSingleQuote,
//     UnterminatedDoubleQuote,
//     UnexpectedCharacter(char),
// }

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            scanner: Scanner::new(input),
            state: LexerState::Normal,
            tokens: Vec::new(),
            current_word: String::new(),
        }
    }

    pub fn lex(mut self) -> Vec<Token> {
        while !self.scanner.is_at_end() {
            match self.state {
                LexerState::Normal => {
                    self.lex_normal();
                }
                LexerState::DoubleQuote => {
                    self.lex_double_quote();
                }
                LexerState::SingleQuote => {
                    self.lex_single_quote();
                }
                LexerState::Escape => {
                    self.lex_escape();
                }
            }
        }

        self.tokens.push(Token::EOF);

        self.tokens
    }

    fn lex_normal(&mut self) {
        let current = self.scanner.current().unwrap();

        match current {
            c if c.is_whitespace() => {
                self.scanner.advance();
            }
            '"' => {
                self.state = LexerState::DoubleQuote;
                self.scanner.advance();
            }

            '|' => {
                self.emit_word();
                self.tokens.push(Token::Pipe);
                self.scanner.advance();
            }
            '>' => {
                let next = self.scanner.peek();
                if next == Some('>') {
                    self.emit_word();
                    self.tokens.push(Token::RedirectAppend);
                    self.scanner.advance();
                }
            }

            '<' => {
                self.emit_word();
                self.tokens.push(Token::RedirectIn);
                self.scanner.advance();
            }

            '>' => {
                self.emit_word();
                self.tokens.push(Token::RedirectOut);
                self.scanner.advance();
            }

            '&' => {
                self.emit_word();
                self.tokens.push(Token::Background);
                self.scanner.advance();
            }

            ';' => {
                self.emit_word();
                self.tokens.push(Token::Semicolon);
                self.scanner.advance();
            }

            '(' => {
                self.emit_word();
                self.tokens.push(Token::LeftParen);
                self.scanner.advance();
            }

            ')' => {
                self.emit_word();
                self.tokens.push(Token::RightParen);
                self.scanner.advance();
            }
            '\'' => {
                self.state = LexerState::SingleQuote;
                self.scanner.advance();
            }
            '\\' => {
                self.state = LexerState::Escape;
                self.scanner.advance(); // consume '\'
            }

            _ => {
                self.lex_word();
            }
        }
    }

    fn lex_word(&mut self) {
        while let Some(c) = self.scanner.current() {
            if c.is_whitespace() {
                break;
            }
            match c {
                '\\' => {
                    self.scanner.advance(); // consume '\'

                    if let Some(next) = self.scanner.current() {
                        self.current_word.push(next);
                        self.scanner.advance();
                    }
                }

                '|' | '<' | '>' | '&' | ';' | '(' | ')' | '\'' | '"' => break,

                _ => {
                    self.current_word.push(c);
                    self.scanner.advance();
                }
            }
        }
        self.emit_word();
    }

    fn lex_single_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            if c == '\'' {
                self.scanner.advance();

                self.state = LexerState::Normal;

                self.emit_word();

                return;
            }

            self.current_word.push(c);
            self.scanner.advance();
        }
        self.emit_word();
    }

    fn lex_double_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            if c == '\"' {
                self.scanner.advance();

                self.state = LexerState::Normal;

                self.emit_word();

                return;
            }

            self.current_word.push(c);
            self.scanner.advance();
        }
        self.emit_word();
    }

    fn lex_escape(&mut self) {
        if let Some(c) = self.scanner.current() {
            self.current_word.push(c);
            self.scanner.advance();
        }

        self.state = LexerState::Normal;
    }

    fn emit_word(&mut self) {
        if !self.current_word.is_empty() {
            self.tokens
                .push(Token::Word(std::mem::take(&mut self.current_word)));
        }
    }
}

use super::scanner::Scanner;
use super::state::LexerState;
use super::token::Token;

pub struct Lexer<'a> {
    scanner: Scanner<'a>,
    state: LexerState,
    tokens: Vec<Token>,
    current_word: String,
    word_started: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            scanner: Scanner::new(input),
            state: LexerState::Normal,
            tokens: Vec::new(),
            current_word: String::new(),
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
                self.word_started = true;
                self.state = LexerState::SingleQuote;
                self.scanner.advance();
            }

            '"' => {
                self.word_started = true;
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
                self.current_word.push(current);
                self.scanner.advance();
            }
        }
    }

    /// Ends the current word, emits an operator token, and consumes its characters.
    fn push_operator(&mut self, token: Token, width: usize) {
        self.emit_word();
        self.tokens.push(token);
        for _ in 0..width {
            self.scanner.advance();
        }
    }

    fn lex_single_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            if c == '\'' {
                self.scanner.advance();
                self.state = LexerState::Normal;
                return;
            }
            self.current_word.push(c);
            self.scanner.advance();
        }
    }

    fn lex_double_quote(&mut self) {
        while let Some(c) = self.scanner.current() {
            if c == '"' {
                self.scanner.advance();
                self.state = LexerState::Normal;
                return;
            }
            self.current_word.push(c);
            self.scanner.advance();
        }
    }

    fn lex_escape(&mut self) {
        if let Some(c) = self.scanner.current() {
            self.current_word.push(c);
            self.scanner.advance();
        }
        self.state = LexerState::Normal;
    }

    fn emit_word(&mut self) {
        if self.word_started {
            self.tokens
                .push(Token::Word(std::mem::take(&mut self.current_word)));
            self.word_started = false;
        }
    }
}
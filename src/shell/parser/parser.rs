use crate::shell::lexer::Token;
use crate::shell::parser::{AST, Redirect, RedirectMode, SimpleCommand};

pub struct Parser {
    pub tokens: Vec<Token>,
    pub position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    pub fn parse(&mut self) -> Result<AST, String> {
        self.parse_simple_command()
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn parse_simple_command(&mut self) -> Result<AST, String> {
        let mut words = Vec::new();
        let mut redirects = Vec::new();

        loop {
            match self.current() {
                Some(Token::Word(word)) => {
                    words.push(word.clone());
                    self.advance();
                }

                Some(Token::RedirectIn) => {
                    self.advance();

                    let filename = self.take_redirect_target()?;

                    redirects.push(Redirect {
                        mode: RedirectMode::Input,
                        target: filename,
                    });
                }

                Some(Token::RedirectOut) => {
                    self.advance();

                    let filename = self.take_redirect_target()?;

                    redirects.push(Redirect {
                        mode: RedirectMode::Output,
                        target: filename,
                    });
                }

                Some(Token::RedirectAppend) => {
                    self.advance();

                    let filename = self.take_redirect_target()?;

                    redirects.push(Redirect {
                        mode: RedirectMode::Append,
                        target: filename,
                    });
                }

                Some(Token::EOF) | None => break,

                Some(_) => {
                    return Err("Unexpected token in simple command".to_string());
                }
            }
        }

        if words.is_empty() {
            return Err("Empty command".to_string());
        }

        let command = words.remove(0);

        Ok(AST::Simple(SimpleCommand {
            command,
            args: words,
            redirects,
        }))
    }

    fn take_redirect_target(&mut self) -> Result<String, String> {
        match self.current() {
            Some(Token::Word(filename)) => {
                let filename = filename.clone();
                self.advance();
                Ok(filename)
            }

            Some(Token::EOF) | None => {
                Err("Syntax error: expected filename after redirection".to_string())
            }

            Some(_) => {
                Err("Syntax error: expected filename after redirection".to_string())
            }
        }
    }
}
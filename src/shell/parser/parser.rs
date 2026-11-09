use crate::shell::lexer::Token;
use crate::shell::parser::{
    AndOr, Connector, ListItem, Pipeline, Redirect, RedirectMode, Separator, SimpleCommand, AST,
};

// Grammar:
//   list        := (and_or (';' | '&'))* and_or? 
//   and_or      := pipeline (('&&' | '||') pipeline)*
//   pipeline    := simple ('|' simple)*
//   simple      := (WORD | redirect)+        (at least one WORD)
//   redirect    := ('<' | '>' | '>>') WORD

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

    pub fn parse(&mut self) -> Result<AST, String> {
        self.parse_list()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.peek(), None | Some(Token::EOF))
    }

    fn parse_list(&mut self) -> Result<AST, String> {
        let mut items = Vec::new();

        while !self.at_end() {
            // A statement cannot start with a separator (e.g. `; ls` or `& &`).
            if matches!(self.peek(), Some(Token::Semicolon | Token::Background)) {
                return Err(syntax_error(self.peek()));
            }

            let and_or = self.parse_and_or()?;

            let separator = match self.peek() {
                Some(Token::Semicolon) => {
                    self.advance();
                    Separator::Sequential
                }
                Some(Token::Background) => {
                    self.advance();
                    Separator::Background
                }
                None | Some(Token::EOF) => Separator::Sequential,
                Some(_) => return Err(syntax_error(self.peek())),
            };

            items.push(ListItem { and_or, separator });
        }

        Ok(AST { items })
    }

    fn parse_and_or(&mut self) -> Result<AndOr, String> {
        let first = self.parse_pipeline()?;
        let mut rest = Vec::new();

        loop {
            let connector = match self.peek() {
                Some(Token::AndIf) => Connector::And,
                Some(Token::OrIf) => Connector::Or,
                _ => break,
            };
            self.advance();
            rest.push((connector, self.parse_pipeline()?));
        }

        Ok(AndOr { first, rest })
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, String> {
        let mut commands = vec![self.parse_simple_command()?];

        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            commands.push(self.parse_simple_command()?);
        }

        Ok(Pipeline { commands })
    }

    fn parse_simple_command(&mut self) -> Result<SimpleCommand, String> {
        let mut words = Vec::new();
        let mut redirects = Vec::new();

        loop {
            let mode = match self.peek() {
                Some(Token::Word(word)) => {
                    words.push(word.clone());
                    self.advance();
                    continue;
                }
                Some(Token::RedirectIn) => RedirectMode::Input,
                Some(Token::RedirectOut) => RedirectMode::Output,
                Some(Token::RedirectAppend) => RedirectMode::Append,
                _ => break,
            };

            self.advance();
            let target = self.take_redirect_target()?;
            redirects.push(Redirect { mode, target });
        }

        if words.is_empty() {
            if self.at_end() {
                return Err("syntax error: expected a command".to_string());
            }
            return Err(syntax_error(self.peek()));
        }

        let command = words.remove(0);

        Ok(SimpleCommand {
            command,
            args: words,
            redirects,
        })
    }

    fn take_redirect_target(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Token::Word(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err("syntax error: expected filename after redirection".to_string()),
        }
    }
}

fn token_text(token: Option<&Token>) -> String {
    match token {
        None | Some(Token::EOF) => "newline".to_string(),
        Some(Token::Word(word)) => word.clone(),
        Some(Token::Pipe) => "|".to_string(),
        Some(Token::AndIf) => "&&".to_string(),
        Some(Token::OrIf) => "||".to_string(),
        Some(Token::RedirectIn) => "<".to_string(),
        Some(Token::RedirectOut) => ">".to_string(),
        Some(Token::RedirectAppend) => ">>".to_string(),
        Some(Token::Background) => "&".to_string(),
        Some(Token::Semicolon) => ";".to_string(),
        Some(Token::LeftParen) => "(".to_string(),
        Some(Token::RightParen) => ")".to_string(),
    }
}

fn syntax_error(token: Option<&Token>) -> String {
    format!("syntax error near unexpected token `{}`", token_text(token))
}
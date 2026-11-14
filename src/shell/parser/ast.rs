use crate::shell::lexer::Word;

// Redirection kinds supported by the shell.
#[derive(Debug, Clone)]
pub enum RedirectMode {
    Input,  // <
    Output, // >
    Append, // >>
    // TODO: stderr redirections (2>, 2>>)
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub mode: RedirectMode,
    pub target: Word,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: String,
    pub value: Word,
}

// A single command: `name args... redirects...`.
#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub assignments: Vec<Assignment>,
    pub words: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

// `cmd1 | cmd2 | cmd3`
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connector {
    And, // &&
    Or,  // ||
}

// `p1 && p2 || p3`, evaluated left to right with short-circuiting.
#[derive(Debug, Clone)]
pub struct AndOr {
    pub first: Pipeline,
    pub rest: Vec<(Connector, Pipeline)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Sequential, // ; or end of line
    Background, // &
}

// One entry of a command list, plus how it is terminated.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub and_or: AndOr,
    pub separator: Separator,
}

// A whole input line.
#[derive(Debug, Clone, Default)]
pub struct AST {
    pub items: Vec<ListItem>,
}
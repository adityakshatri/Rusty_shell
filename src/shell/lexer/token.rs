#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),

    Pipe,          // |
    AndIf,         // &&
    OrIf,          // ||

    RedirectIn,    // <
    RedirectOut,   // >
    RedirectAppend,// >>

    Background,    // &
    Semicolon,     // ;
    LeftParen,     // (
    RightParen,    // )

    EOF,
}

//basic token types
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),

    Pipe,
//TODO: support stderr stdin and following redirections
    RedirectIn,         // <

    RedirectOut, 
    
    RedirectAppend,       // <

    Background,

    Semicolon,

    LeftParen,

    RightParen,

    Ampersand,

    EOF,
}

//basic token types
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),

    Pipe,

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
//what each macro does is explained in the following link: https://doc.rust-lang.org/reference/attributes/derive.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quoting {
    Single,
    Double,
    Unquoted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordPart {
    pub quoting: Quoting,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Word {
    pub parts: Vec<WordPart>,
}

impl Word {
    pub fn raw_text(&self) -> String {
        self.parts.iter().map(|part| part.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(Word),

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
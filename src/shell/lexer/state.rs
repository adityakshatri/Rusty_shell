#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexerState {
    Normal,
    SingleQuote,
    DoubleQuote,
    Escape,
}
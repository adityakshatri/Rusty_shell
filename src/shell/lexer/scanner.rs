pub struct Scanner<'a> {
    input: &'a str,
    chars: Vec<char>,
    position: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            position: 0,
        }
    }
    pub fn current(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }
    pub fn advance(&mut self) {
        if self.position < self.chars.len() {
            self.position += 1;
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }
    pub fn position(&self) -> usize {
        self.position
    }
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

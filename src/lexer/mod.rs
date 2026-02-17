pub mod token;
use token::Token;

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input.get(self.position).cloned()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.position;

        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let num: String = self.input[start..self.position].iter().collect();
        Token::Number(num.parse().unwrap())
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;

        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let ident: String = self.input[start..self.position].iter().collect();

        match ident.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "return" => Token::Return,
            "intent" => Token::Intent,
            "for" => Token::For,
            "in" => Token::In,
            "speed" => Token::Speed,
            "parallel" => Token::Parallel,
            "memory_low" => Token::MemoryLow,
            _ => Token::Identifier(ident),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.current_char() {
            Some('{') => { self.advance(); Token::LBrace }
            Some('}') => { self.advance(); Token::RBrace }
            Some('(') => { self.advance(); Token::LParen }
            Some(')') => { self.advance(); Token::RParen }
            Some(',') => { self.advance(); Token::Comma }
            Some('+') => { self.advance(); Token::Plus }
            Some('-') => { self.advance(); Token::Minus }
            Some('*') => { self.advance(); Token::Star }
            Some('/') => { self.advance(); Token::Slash }
            Some('=') => { self.advance(); Token::Equal }

            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() => self.read_identifier(),

            Some(_) => {
                self.advance();
                self.next_token()
            }

            None => Token::EOF,
        }
    }
}

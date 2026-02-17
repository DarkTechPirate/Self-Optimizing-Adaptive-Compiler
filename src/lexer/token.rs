#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // keywords
    Fn,
    Let,
    Return,
    Intent,
    For,
    In,

    // intent keywords
    Speed,
    Parallel,
    MemoryLow,

    // symbols
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,

    // literals
    Number(i64),
    Identifier(String),

    EOF,
}

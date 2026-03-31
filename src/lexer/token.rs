#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // keywords
    Fn,
    Let,
    Return,
    Intent,
    For,
    In,
    While,
    If,
    Else,

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
    EqualEqual,  // ==
    NotEqual,    // !=
    Less,        // <
    LessEqual,   // <=
    Greater,     // >
    GreaterEqual,// >=
    DotDot,      // .. (range)

    // literals
    Number(i64),
    Identifier(String),

    EOF,
}

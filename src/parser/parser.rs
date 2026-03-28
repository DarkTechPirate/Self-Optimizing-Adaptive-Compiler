use crate::lexer::token::Token;
use crate::ast::node::*;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    pub fn parse_program(&mut self) -> Program {
        let mut functions = Vec::new();

        while *self.current() != Token::EOF {
            functions.push(self.parse_function());
        }

        Program { functions }
    }

    fn parse_function(&mut self) -> Function {
        // expect fn
        self.advance();

        let name = if let Token::Identifier(n) = self.current() {
            n.clone()
        } else {
            panic!("Expected function name");
        };

        self.advance(); // name
        self.advance(); // (

        let mut params = Vec::new();
        while *self.current() != Token::RParen {
            if let Token::Identifier(p) = self.current() {
                params.push(p.clone());
            }
            self.advance();
        }

        self.advance(); // )
        self.advance(); // {

        let body = self.parse_block();

        self.advance(); // }

        Function { name, params, body }
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while *self.current() != Token::RBrace && *self.current() != Token::EOF {
            stmts.push(self.parse_statement());
        }
        stmts
    }

    fn parse_statement(&mut self) -> Stmt {
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Return => self.parse_return(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::If => self.parse_if(),
            Token::Identifier(_) => self.parse_assign_or_expr(),
            _ => panic!("Unknown statement: {:?}", self.current()),
        }
    }

    fn parse_let(&mut self) -> Stmt {
        self.advance(); // let

        let name = if let Token::Identifier(n) = self.current() {
            n.clone()
        } else {
            panic!("Expected identifier");
        };

        self.advance(); // name
        self.advance(); // =

        let expr = self.parse_expr();

        Stmt::Let { name, value: expr }
    }

    fn parse_return(&mut self) -> Stmt {
        self.advance(); // return
        let expr = self.parse_expr();
        Stmt::Return(expr)
    }

    fn parse_while(&mut self) -> Stmt {
        self.advance(); // while

        let condition = self.parse_expr();

        self.advance(); // {
        let body = self.parse_block();
        self.advance(); // }

        Stmt::While { condition, body }
    }

    fn parse_for(&mut self) -> Stmt {
        self.advance(); // for

        let var = if let Token::Identifier(n) = self.current() {
            n.clone()
        } else {
            panic!("Expected loop variable");
        };
        self.advance(); // var

        self.advance(); // in

        let start = self.parse_primary();

        self.advance(); // ..

        let end = self.parse_primary();

        self.advance(); // {
        let body = self.parse_block();
        self.advance(); // }

        Stmt::For { var, start, end, body }
    }

    fn parse_if(&mut self) -> Stmt {
        self.advance(); // if

        let condition = self.parse_expr();

        self.advance(); // {
        let then_body = self.parse_block();
        self.advance(); // }

        let else_body = if *self.current() == Token::Else {
            self.advance(); // else
            self.advance(); // {
            let body = self.parse_block();
            self.advance(); // }
            Some(body)
        } else {
            None
        };

        Stmt::If { condition, then_body, else_body }
    }

    fn parse_assign_or_expr(&mut self) -> Stmt {
        let name = if let Token::Identifier(n) = self.current() {
            n.clone()
        } else {
            panic!("Expected identifier");
        };
        self.advance();

        if *self.current() == Token::Equal {
            self.advance(); // =
            let value = self.parse_expr();
            Stmt::Assign { name, value }
        } else {
            // It's an expression starting with identifier
            Stmt::Expr(Expr::Identifier(name))
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Expr {
        let left = self.parse_additive();

        match self.current() {
            Token::EqualEqual | Token::NotEqual | 
            Token::Less | Token::LessEqual | 
            Token::Greater | Token::GreaterEqual => {
                let op = match self.current() {
                    Token::EqualEqual => "==",
                    Token::NotEqual => "!=",
                    Token::Less => "<",
                    Token::LessEqual => "<=",
                    Token::Greater => ">",
                    Token::GreaterEqual => ">=",
                    _ => unreachable!(),
                }.to_string();
                self.advance();
                let right = self.parse_additive();
                Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                }
            }
            _ => left,
        }
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();

        loop {
            match self.current() {
                Token::Plus | Token::Minus => {
                    let op = if *self.current() == Token::Plus { "+" } else { "-" }.to_string();
                    self.advance();
                    let right = self.parse_multiplicative();
                    left = Expr::Binary {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_primary();

        loop {
            match self.current() {
                Token::Star | Token::Slash => {
                    let op = if *self.current() == Token::Star { "*" } else { "/" }.to_string();
                    self.advance();
                    let right = self.parse_primary();
                    left = Expr::Binary {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        left
    }

    fn parse_primary(&mut self) -> Expr {
        match self.current() {
            Token::Number(n) => {
                let v = *n;
                self.advance();
                Expr::Number(v)
            }
            Token::Identifier(name) => {
                let n = name.clone();
                self.advance();
                Expr::Identifier(n)
            }
            Token::LParen => {
                self.advance(); // (
                let expr = self.parse_expr();
                self.advance(); // )
                expr
            }
            _ => panic!("Unexpected token in expression: {:?}", self.current()),
        }
    }
}

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
        self.tokens.get(self.position).unwrap()
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

        let mut body = Vec::new();
        while *self.current() != Token::RBrace {
            body.push(self.parse_statement());
        }

        self.advance(); // }

        Function { name, params, body }
    }

    fn parse_statement(&mut self) -> Stmt {
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Return => self.parse_return(),
            _ => panic!("Unknown statement"),
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

    fn parse_expr(&mut self) -> Expr {
        let left = self.parse_primary();

        if let Token::Plus = self.current() {
            self.advance();
            let right = self.parse_primary();

            return Expr::Binary {
                left: Box::new(left),
                op: "+".to_string(),
                right: Box::new(right),
            };
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
            _ => panic!("Unexpected token in expression"),
        }
    }
}

mod lexer;
mod parser;
mod ast;
mod ir;
mod vm;
mod optimizer;

use lexer::Lexer;
use lexer::token::Token;
use parser::parser::Parser;
use ir::lower::Lowerer;
use vm::vm::NyxVM;
use optimizer::Optimizer;

fn main() {
    let source = r#"
        fn sum() {
            let x = 5 + 3
            return x
        }
    "#;

    // LEXER
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        tokens.push(tok.clone());
        if tok == Token::EOF { break; }
    }

    // PARSER
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();

    // AST → IR
    let mut ir_program = Lowerer::lower_program(program);

    println!("\n=== First Run ===");
    let mut vm = NyxVM::new();
    vm.run_program(&mut ir_program);

    // Analyze
    Optimizer::analyze(&ir_program);

    // Optimize
    Optimizer::optimize(&mut ir_program);

    println!("\n=== Recompiled Run ===");
    let mut vm2 = NyxVM::new();
    vm2.run_program(&mut ir_program);
}

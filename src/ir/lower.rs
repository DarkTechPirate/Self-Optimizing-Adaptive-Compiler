use crate::ast::node::*;
use super::ir::*;

pub struct Lowerer;

impl Lowerer {
    pub fn lower_program(program: Program) -> ProgramIR {
        let mut functions = Vec::new();

        for f in program.functions {
            functions.push(Self::lower_function(f));
        }

        ProgramIR { functions }
    }

    fn lower_function(func: Function) -> FunctionIR {
        let mut instructions = Vec::new();

        for stmt in func.body {
            Self::lower_stmt(stmt, &mut instructions);
        }

        let block = BasicBlock { instructions };

        FunctionIR {
            name: func.name,
            params: func.params,
            blocks: vec![block],
            intents: vec![IntentTag::Speed],
        }
    }

    fn lower_stmt(stmt: Stmt, instructions: &mut Vec<Instruction>) {
        match stmt {
            Stmt::Let { name, value } => {
                let val = Self::lower_expr(value, instructions);

                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![val],
                    result: Some(name),
                    intents: vec![],
                    profile: ProfileData { exec_count: 0 },
                });
            }

            Stmt::Return(expr) => {
                let val = Self::lower_expr(expr, instructions);

                instructions.push(Instruction {
                    opcode: OpCode::Return,
                    operands: vec![val],
                    result: None,
                    intents: vec![],
                    profile: ProfileData { exec_count: 0 },
                });
            }

            _ => {}
        }
    }

    fn lower_expr(expr: Expr, instructions: &mut Vec<Instruction>) -> String {
        match expr {
            Expr::Number(n) => {
                let temp = format!("t{}", instructions.len());

                instructions.push(Instruction {
                    opcode: OpCode::LoadConst,
                    operands: vec![n.to_string()],
                    result: Some(temp.clone()),
                    intents: vec![],
                    profile: ProfileData { exec_count: 0 },
                });

                temp
            }

            Expr::Identifier(name) => name,

            Expr::Binary { left, op: _, right } => {
                let l = Self::lower_expr(*left, instructions);
                let r = Self::lower_expr(*right, instructions);

                let temp = format!("t{}", instructions.len());

                instructions.push(Instruction {
                    opcode: OpCode::Add,
                    operands: vec![l, r],
                    result: Some(temp.clone()),
                    intents: vec![],
                    profile: ProfileData { exec_count: 0 },
                });

                temp
            }
        }
    }
}

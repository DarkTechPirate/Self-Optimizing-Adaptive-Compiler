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

        let block = BasicBlock {
            label: Some(format!("{}_entry", func.name)),
            instructions,
        };

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

            Stmt::Expr(expr) => {
                Self::lower_expr(expr, instructions);
            }
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

            Expr::Binary { left, op, right } => {
                let l = Self::lower_expr(*left, instructions);
                let r = Self::lower_expr(*right, instructions);

                let temp = format!("t{}", instructions.len());

                let opcode = match op.as_str() {
                    "+" => OpCode::Add,
                    "-" => OpCode::Sub,
                    "*" => OpCode::Mul,
                    "/" => OpCode::Div,
                    "%" => OpCode::Mod,
                    "==" => OpCode::CmpEq,
                    "!=" => OpCode::CmpNe,
                    "<" => OpCode::CmpLt,
                    "<=" => OpCode::CmpLe,
                    ">" => OpCode::CmpGt,
                    ">=" => OpCode::CmpGe,
                    _ => OpCode::Add, // fallback
                };

                instructions.push(Instruction {
                    opcode,
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

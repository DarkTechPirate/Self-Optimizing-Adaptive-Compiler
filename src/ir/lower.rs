use crate::ast::node::*;
use super::ir::*;

pub struct Lowerer {
    label_counter: usize,
}

impl Lowerer {
    pub fn lower_program(program: Program) -> ProgramIR {
        let mut lowerer = Lowerer { label_counter: 0 };
        let mut functions = Vec::new();

        for f in program.functions {
            functions.push(lowerer.lower_function(f));
        }

        ProgramIR { functions }
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    fn lower_function(&mut self, func: Function) -> FunctionIR {
        let mut instructions = Vec::new();

        for stmt in func.body {
            self.lower_stmt(stmt, &mut instructions);
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

    fn lower_stmt(&mut self, stmt: Stmt, instructions: &mut Vec<Instruction>) {
        match stmt {
            Stmt::Let { name, value } => {
                let val = self.lower_expr(value, instructions);

                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![val],
                    result: Some(name),
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }

            Stmt::Assign { name, value } => {
                let val = self.lower_expr(value, instructions);

                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![val],
                    result: Some(name),
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }

            Stmt::Return(expr) => {
                let val = self.lower_expr(expr, instructions);

                instructions.push(Instruction {
                    opcode: OpCode::Return,
                    operands: vec![val],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }

            Stmt::Expr(expr) => {
                self.lower_expr(expr, instructions);
            }

            Stmt::While { condition, body } => {
                let loop_start = self.new_label("while_start");
                let loop_end = self.new_label("while_end");

                // Label: loop_start
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![loop_start.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Evaluate condition
                let cond = self.lower_expr(condition, instructions);

                // Branch to end if condition is false (cond == 0)
                let not_cond = format!("t{}", instructions.len());
                instructions.push(Instruction {
                    opcode: OpCode::CmpEq,
                    operands: vec![cond, "0".to_string()],
                    result: Some(not_cond.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                instructions.push(Instruction {
                    opcode: OpCode::Branch,
                    operands: vec![not_cond, loop_end.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Body
                for s in body {
                    self.lower_stmt(s, instructions);
                }

                // Jump back to start
                instructions.push(Instruction {
                    opcode: OpCode::Jump,
                    operands: vec![loop_start],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Label: loop_end
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![loop_end],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }

            Stmt::For { var, start, end, body } => {
                let loop_start = self.new_label("for_start");
                let loop_end = self.new_label("for_end");

                // Initialize loop variable
                let start_val = self.lower_expr(start, instructions);
                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![start_val],
                    result: Some(var.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Store end value
                let end_val = self.lower_expr(end, instructions);
                let end_var = format!("_for_end_{}", self.label_counter);
                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![end_val],
                    result: Some(end_var.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Label: loop_start
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![loop_start.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Check if var < end
                let cmp_result = format!("t{}", instructions.len());
                instructions.push(Instruction {
                    opcode: OpCode::CmpLt,
                    operands: vec![var.clone(), end_var],
                    result: Some(cmp_result.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Branch to end if not (cmp_result == 0)
                let not_cond = format!("t{}", instructions.len());
                instructions.push(Instruction {
                    opcode: OpCode::CmpEq,
                    operands: vec![cmp_result, "0".to_string()],
                    result: Some(not_cond.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                instructions.push(Instruction {
                    opcode: OpCode::Branch,
                    operands: vec![not_cond, loop_end.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Body
                for s in body {
                    self.lower_stmt(s, instructions);
                }

                // Increment loop variable
                let inc_result = format!("t{}", instructions.len());
                instructions.push(Instruction {
                    opcode: OpCode::Add,
                    operands: vec![var.clone(), "1".to_string()],
                    result: Some(inc_result.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                instructions.push(Instruction {
                    opcode: OpCode::StoreVar,
                    operands: vec![inc_result],
                    result: Some(var),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Jump back to start
                instructions.push(Instruction {
                    opcode: OpCode::Jump,
                    operands: vec![loop_start],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Label: loop_end
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![loop_end],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }

            Stmt::If { condition, then_body, else_body } => {
                let else_label = self.new_label("if_else");
                let end_label = self.new_label("if_end");

                // Evaluate condition
                let cond = self.lower_expr(condition, instructions);

                // Branch to else if condition is false
                let not_cond = format!("t{}", instructions.len());
                instructions.push(Instruction {
                    opcode: OpCode::CmpEq,
                    operands: vec![cond, "0".to_string()],
                    result: Some(not_cond.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                instructions.push(Instruction {
                    opcode: OpCode::Branch,
                    operands: vec![not_cond, else_label.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Then body
                for s in then_body {
                    self.lower_stmt(s, instructions);
                }

                // Jump to end
                instructions.push(Instruction {
                    opcode: OpCode::Jump,
                    operands: vec![end_label.clone()],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Else label
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![else_label],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                // Else body
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.lower_stmt(s, instructions);
                    }
                }

                // End label
                instructions.push(Instruction {
                    opcode: OpCode::Label,
                    operands: vec![end_label],
                    result: None,
                    intents: vec![],
                    profile: ProfileData::new(),
                });
            }
        }
    }

    fn lower_expr(&mut self, expr: Expr, instructions: &mut Vec<Instruction>) -> String {
        match expr {
            Expr::Number(n) => {
                let temp = format!("t{}", instructions.len());

                instructions.push(Instruction {
                    opcode: OpCode::LoadConst,
                    operands: vec![n.to_string()],
                    result: Some(temp.clone()),
                    intents: vec![],
                    profile: ProfileData::new(),
                });

                temp
            }

            Expr::Identifier(name) => name,

            Expr::Binary { left, op, right } => {
                let l = self.lower_expr(*left, instructions);
                let r = self.lower_expr(*right, instructions);

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
                    profile: ProfileData::new(),
                });

                temp
            }
        }
    }
}

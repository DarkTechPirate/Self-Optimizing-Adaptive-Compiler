use std::collections::{HashMap, HashSet};
use crate::ir::ir::*;

pub struct Optimizer;

impl Optimizer {
    pub fn analyze(program: &ProgramIR) {
        println!("\n=== Optimizer Analysis ===");

        for func in &program.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if instr.profile.exec_count > 0 {
                        println!("Hot instruction detected: {:?}", instr.opcode);
                    }
                }
            }
        }
    }

    /// Run all optimization passes
    pub fn optimize(program: &mut ProgramIR) {
        println!("\n=== Optimization Pass ===");

        Self::constant_folding(program);
        Self::dead_code_elimination(program);
    }

    /// Helper: get constant value from operand
    fn get_const(op: &str, constants: &HashMap<String, i64>) -> Option<i64> {
        if let Ok(n) = op.parse::<i64>() {
            Some(n)
        } else {
            constants.get(op).cloned()
        }
    }

    /// Optimization: Constant Propagation + Folding for all arithmetic ops
    fn constant_folding(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut constants: HashMap<String, i64> = HashMap::new();

                for instr in &mut block.instructions {
                    // Track constants from LoadConst
                    if instr.opcode == OpCode::LoadConst {
                        if let Some(res) = &instr.result {
                            if let Ok(val) = instr.operands[0].parse::<i64>() {
                                constants.insert(res.clone(), val);
                            }
                        }
                        continue;
                    }

                    // Try to fold binary arithmetic operations
                    let folded = match instr.opcode {
                        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => {
                            let v1 = Self::get_const(&instr.operands[0], &constants);
                            let v2 = Self::get_const(&instr.operands[1], &constants);

                            if let (Some(a), Some(b)) = (v1, v2) {
                                let result = match instr.opcode {
                                    OpCode::Add => a + b,
                                    OpCode::Sub => a - b,
                                    OpCode::Mul => a * b,
                                    OpCode::Div => if b != 0 { a / b } else { 0 },
                                    OpCode::Mod => if b != 0 { a % b } else { 0 },
                                    _ => unreachable!(),
                                };
                                let op_str = match instr.opcode {
                                    OpCode::Add => "+",
                                    OpCode::Sub => "-",
                                    OpCode::Mul => "*",
                                    OpCode::Div => "/",
                                    OpCode::Mod => "%",
                                    _ => "?",
                                };
                                println!("[Constant Fold] {} {} {} -> {}", a, op_str, b, result);
                                Some(result)
                            } else {
                                None
                            }
                        }
                        OpCode::CmpEq | OpCode::CmpNe | OpCode::CmpLt | 
                        OpCode::CmpLe | OpCode::CmpGt | OpCode::CmpGe => {
                            let v1 = Self::get_const(&instr.operands[0], &constants);
                            let v2 = Self::get_const(&instr.operands[1], &constants);

                            if let (Some(a), Some(b)) = (v1, v2) {
                                let result = match instr.opcode {
                                    OpCode::CmpEq => if a == b { 1 } else { 0 },
                                    OpCode::CmpNe => if a != b { 1 } else { 0 },
                                    OpCode::CmpLt => if a < b { 1 } else { 0 },
                                    OpCode::CmpLe => if a <= b { 1 } else { 0 },
                                    OpCode::CmpGt => if a > b { 1 } else { 0 },
                                    OpCode::CmpGe => if a >= b { 1 } else { 0 },
                                    _ => unreachable!(),
                                };
                                let op_str = match instr.opcode {
                                    OpCode::CmpEq => "==",
                                    OpCode::CmpNe => "!=",
                                    OpCode::CmpLt => "<",
                                    OpCode::CmpLe => "<=",
                                    OpCode::CmpGt => ">",
                                    OpCode::CmpGe => ">=",
                                    _ => "?",
                                };
                                println!("[Constant Fold] {} {} {} -> {}", a, op_str, b, result);
                                Some(result)
                            } else {
                                None
                            }
                        }
                        OpCode::Neg => {
                            let v = Self::get_const(&instr.operands[0], &constants);
                            if let Some(a) = v {
                                println!("[Constant Fold] -{} -> {}", a, -a);
                                Some(-a)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    // If we folded, convert to LoadConst
                    if let Some(result) = folded {
                        instr.opcode = OpCode::LoadConst;
                        instr.operands = vec![result.to_string()];
                        if let Some(res) = &instr.result {
                            constants.insert(res.clone(), result);
                        }
                    }
                }
            }
        }
    }

    /// Optimization: Dead Code Elimination
    fn dead_code_elimination(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                // Step 1: Find all used variables (backward analysis)
                let mut used_vars: HashSet<String> = HashSet::new();

                for instr in block.instructions.iter().rev() {
                    match instr.opcode {
                        OpCode::Return | OpCode::Branch | OpCode::Call => {
                            for op in &instr.operands {
                                if op.parse::<i64>().is_err() {
                                    used_vars.insert(op.clone());
                                }
                            }
                        }
                        _ => {
                            let is_used = instr.result.as_ref()
                                .map(|r| used_vars.contains(r))
                                .unwrap_or(false);

                            if is_used {
                                for op in &instr.operands {
                                    if op.parse::<i64>().is_err() {
                                        used_vars.insert(op.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // Step 2: Remove dead instructions
                let original_count = block.instructions.len();
                let mut seen_return = false;

                block.instructions.retain(|instr| {
                    if seen_return {
                        println!("[DCE] Removed unreachable: {:?}", instr.opcode);
                        return false;
                    }

                    if instr.opcode == OpCode::Return {
                        seen_return = true;
                        return true;
                    }

                    // Instructions with side effects
                    let has_side_effect = matches!(
                        instr.opcode,
                        OpCode::Return | OpCode::StoreVar | OpCode::Call | 
                        OpCode::Jump | OpCode::Branch | OpCode::Label
                    );
                    
                    let is_used = instr.result.as_ref()
                        .map(|r| used_vars.contains(r))
                        .unwrap_or(false);

                    if !has_side_effect && !is_used {
                        println!("[DCE] Removed dead code: {:?} -> {:?}", instr.opcode, instr.result);
                        return false;
                    }

                    true
                });

                let removed = original_count - block.instructions.len();
                if removed > 0 {
                    println!("[DCE] Eliminated {} dead instruction(s)", removed);
                }
            }
        }
    }
}

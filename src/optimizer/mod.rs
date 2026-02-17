use std::collections::HashMap;
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

    // Optimization: Constant Propagation + Folding
    pub fn optimize(program: &mut ProgramIR) {
        println!("\n=== Optimization Pass ===");

        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut constants = HashMap::new();

                for instr in &mut block.instructions {
                    // Track constants from LoadConst
                    if let OpCode::LoadConst = instr.opcode {
                        if let Some(res) = &instr.result {
                            // If operand is a direct number
                            if let Ok(val) = instr.operands[0].parse::<i64>() {
                                constants.insert(res.clone(), val);
                            }
                        }
                    }

                    // Try to fold Add
                    if let OpCode::Add = instr.opcode {
                        let op1 = &instr.operands[0];
                        let op2 = &instr.operands[1];

                        // Check if both operands are known constants
                        let v1 = if let Ok(n) = op1.parse::<i64>() { Some(n) } else { constants.get(op1).cloned() };
                        let v2 = if let Ok(n) = op2.parse::<i64>() { Some(n) } else { constants.get(op2).cloned() };

                        if let (Some(a), Some(b)) = (v1, v2) {
                            let result = a + b;
                            println!("Constant folded Add {} + {} -> {}", a, b, result);

                            instr.opcode = OpCode::LoadConst;
                            instr.operands = vec![result.to_string()];
                            // Update our knowledge for future instructions
                            if let Some(res) = &instr.result {
                                constants.insert(res.clone(), result);
                            }
                        }
                    }
                }
            }
        }
    }
}

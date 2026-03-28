use std::collections::{HashMap, HashSet};
use crate::ir::ir::*;

pub struct Optimizer;

impl Optimizer {
    pub fn analyze(program: &ProgramIR) {
        println!("\n=== Optimizer Analysis ===");

        let mut hot_loops = 0;
        for func in &program.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if instr.profile.exec_count > 1 {
                        println!("Hot instruction ({} execs): {:?}", instr.profile.exec_count, instr.opcode);
                        if instr.opcode == OpCode::Label && instr.operands.first().map(|s| s.contains("for") || s.contains("while")).unwrap_or(false) {
                            hot_loops += 1;
                        }
                    }
                }
            }
        }
        if hot_loops > 0 {
            println!("Detected {} hot loop(s) - candidates for optimization", hot_loops);
        }
    }

    /// Run all optimization passes
    pub fn optimize(program: &mut ProgramIR) {
        println!("\n=== Optimization Pass ===");

        Self::constant_folding(program);
        Self::loop_invariant_code_motion(program);
        Self::strength_reduction(program);
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
                
                // Also find loop variables that should never be removed
                let mut loop_vars: HashSet<String> = HashSet::new();
                for instr in &block.instructions {
                    // CmpLt typically compares loop var to end
                    if instr.opcode == OpCode::CmpLt || instr.opcode == OpCode::CmpLe {
                        for op in &instr.operands {
                            if op.parse::<i64>().is_err() {
                                loop_vars.insert(op.clone());
                            }
                        }
                    }
                }

                for instr in block.instructions.iter().rev() {
                    match instr.opcode {
                        OpCode::Return | OpCode::Branch | OpCode::Call | 
                        OpCode::CmpLt | OpCode::CmpLe | OpCode::CmpGt | 
                        OpCode::CmpGe | OpCode::CmpEq | OpCode::CmpNe => {
                            for op in &instr.operands {
                                if op.parse::<i64>().is_err() {
                                    used_vars.insert(op.clone());
                                }
                            }
                            // Also mark result as used for comparisons
                            if let Some(res) = &instr.result {
                                used_vars.insert(res.clone());
                            }
                        }
                        OpCode::StoreVar => {
                            // If storing to a loop variable, mark operands as used
                            if let Some(res) = &instr.result {
                                if loop_vars.contains(res) || used_vars.contains(res) {
                                    for op in &instr.operands {
                                        if op.parse::<i64>().is_err() {
                                            used_vars.insert(op.clone());
                                        }
                                    }
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

    /// Loop Invariant Code Motion (LICM)
    /// Moves computations that don't change inside a loop to before the loop
    fn loop_invariant_code_motion(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                // Find loop regions (between loop_start and loop_end labels)
                let mut i = 0;
                while i < block.instructions.len() {
                    if block.instructions[i].opcode == OpCode::Label {
                        let label = block.instructions[i].operands.first().cloned().unwrap_or_default();
                        
                        if label.contains("for_start") || label.contains("while_start") {
                            // Find matching end label
                            let end_label = label.replace("_start", "_end");
                            let mut end_idx = None;
                            
                            for j in (i + 1)..block.instructions.len() {
                                if block.instructions[j].opcode == OpCode::Label {
                                    if block.instructions[j].operands.first() == Some(&end_label) {
                                        end_idx = Some(j);
                                        break;
                                    }
                                }
                            }
                            
                            if let Some(end) = end_idx {
                                // Collect variables modified in loop
                                let mut modified_in_loop: HashSet<String> = HashSet::new();
                                for j in i..=end {
                                    if let Some(res) = &block.instructions[j].result {
                                        modified_in_loop.insert(res.clone());
                                    }
                                }
                                
                                // Find invariant LoadConst instructions that can be hoisted
                                let mut to_hoist: Vec<usize> = Vec::new();
                                for j in (i + 1)..end {
                                    let instr = &block.instructions[j];
                                    if instr.opcode == OpCode::LoadConst {
                                        // Check if result is only used, not modified elsewhere
                                        if let Some(res) = &instr.result {
                                            // Only hoist if it's a simple constant load
                                            let uses_count = block.instructions[i..=end].iter()
                                                .filter(|ins| ins.operands.contains(res))
                                                .count();
                                            if uses_count > 1 {
                                                to_hoist.push(j);
                                            }
                                        }
                                    }
                                }
                                
                                // Hoist instructions (move before loop start)
                                for (offset, &idx) in to_hoist.iter().enumerate() {
                                    let actual_idx = idx - offset;
                                    let hoisted = block.instructions.remove(actual_idx);
                                    println!("[LICM] Hoisted {:?} out of loop", hoisted.opcode);
                                    block.instructions.insert(i, hoisted);
                                }
                            }
                        }
                    }
                    i += 1;
                }
            }
        }
    }

    /// Strength Reduction
    /// Replaces expensive operations with cheaper ones (e.g., x * 2 -> x + x)
    fn strength_reduction(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instructions {
                    match instr.opcode {
                        OpCode::Mul => {
                            // x * 2 -> x + x
                            if instr.operands.len() == 2 {
                                if instr.operands[1] == "2" {
                                    println!("[Strength Reduction] {} * 2 -> {} + {}", 
                                        instr.operands[0], instr.operands[0], instr.operands[0]);
                                    instr.opcode = OpCode::Add;
                                    let op = instr.operands[0].clone();
                                    instr.operands = vec![op.clone(), op];
                                } else if instr.operands[0] == "2" {
                                    println!("[Strength Reduction] 2 * {} -> {} + {}", 
                                        instr.operands[1], instr.operands[1], instr.operands[1]);
                                    instr.opcode = OpCode::Add;
                                    let op = instr.operands[1].clone();
                                    instr.operands = vec![op.clone(), op];
                                }
                            }
                        }
                        OpCode::Div => {
                            // x / 1 -> x (copy)
                            if instr.operands.len() == 2 && instr.operands[1] == "1" {
                                println!("[Strength Reduction] {} / 1 -> copy", instr.operands[0]);
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[0].clone()];
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

use std::collections::{HashMap, HashSet};
use crate::ir::ir::*;

#[derive(Debug, Clone)]
pub struct OptimizationPlan {
    pub constant_folding: bool,
    pub dead_code_elimination: bool,
    pub loop_invariant_code_motion: bool,
    pub strength_reduction: bool,
    pub common_subexpression_elimination: bool,
    pub peephole: bool,
    pub loop_unrolling: bool,
    pub inline_function: bool,
    pub vectorize: bool,
}

impl OptimizationPlan {
    pub fn baseline() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
            loop_invariant_code_motion: false,
            strength_reduction: false,
            common_subexpression_elimination: true,
            peephole: true,
            loop_unrolling: false,
            inline_function: false,
            vectorize: false,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
            loop_invariant_code_motion: true,
            strength_reduction: true,
            common_subexpression_elimination: true,
            peephole: true,
            loop_unrolling: true,
            inline_function: true,
            vectorize: true,
        }
    }

    pub fn from_strategies(strategies: &[String]) -> Self {
        let mut plan = Self {
            constant_folding: false,
            dead_code_elimination: false,
            loop_invariant_code_motion: false,
            strength_reduction: false,
            common_subexpression_elimination: false,
            peephole: false,
            loop_unrolling: false,
            inline_function: false,
            vectorize: false,
        };

        for strategy in strategies {
            let s = strategy.to_lowercase();

            if s.contains("constant") {
                plan.constant_folding = true;
            }
            if s.contains("dead code") || s.contains("dead_code") {
                plan.dead_code_elimination = true;
            }
            if s.contains("loop invariant") || s.contains("loop_invariant") {
                plan.loop_invariant_code_motion = true;
            }
            if s.contains("strength") {
                plan.strength_reduction = true;
            }
            if s.contains("cse") || s.contains("common_subexpression") || s.contains("common subexpression") {
                plan.common_subexpression_elimination = true;
            }
            if s.contains("peephole") {
                plan.peephole = true;
            }
            if s.contains("unroll") {
                plan.loop_unrolling = true;
            }
            if s.contains("inline") {
                plan.inline_function = true;
            }
            if s.contains("vector") {
                plan.vectorize = true;
            }
        }

        if !plan.has_enabled_passes() {
            Self::baseline()
        } else {
            plan
        }
    }

    pub fn has_enabled_passes(&self) -> bool {
        self.constant_folding
            || self.dead_code_elimination
            || self.loop_invariant_code_motion
            || self.strength_reduction
            || self.common_subexpression_elimination
            || self.peephole
            || self.loop_unrolling
            || self.inline_function
            || self.vectorize
    }

    pub fn enabled_passes(&self) -> Vec<String> {
        let mut passes = Vec::new();

        if self.constant_folding {
            passes.push("constant_folding".to_string());
        }
        if self.dead_code_elimination {
            passes.push("dead_code_elimination".to_string());
        }
        if self.loop_invariant_code_motion {
            passes.push("loop_invariant_code_motion".to_string());
        }
        if self.strength_reduction {
            passes.push("strength_reduction".to_string());
        }
        if self.common_subexpression_elimination {
            passes.push("common_subexpression_elimination".to_string());
        }
        if self.peephole {
            passes.push("peephole".to_string());
        }
        if self.loop_unrolling {
            passes.push("loop_unrolling".to_string());
        }
        if self.inline_function {
            passes.push("inline_function".to_string());
        }
        if self.vectorize {
            passes.push("vectorize".to_string());
        }

        passes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OperandKey {
    Const(String),
    Var(String, u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExprKey {
    opcode: OpCode,
    op1: OperandKey,
    op2: Option<OperandKey>,
}

#[derive(Debug, Clone)]
struct ExprValue {
    result: String,
    version: u64,
}

pub struct Optimizer;

impl Optimizer {
    pub fn analyze(program: &ProgramIR) {
        eprintln!("\n=== Optimizer Analysis ===");

        let mut hot_loops = 0;
        for func in &program.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if instr.profile.exec_count > 1 {
                        eprintln!("Hot instruction ({} execs): {:?}", instr.profile.exec_count, instr.opcode);
                        if instr.opcode == OpCode::Label && instr.operands.first().map(|s| s.contains("for") || s.contains("while")).unwrap_or(false) {
                            hot_loops += 1;
                        }
                    }
                }
            }
        }
        if hot_loops > 0 {
            eprintln!("Detected {} hot loop(s) - candidates for optimization", hot_loops);
        }
    }

    /// Run all optimization passes
    pub fn optimize(program: &mut ProgramIR) -> Vec<String> {
        Self::optimize_with_plan(program, &OptimizationPlan::aggressive())
    }

    /// Run selected optimization passes
    pub fn optimize_with_plan(program: &mut ProgramIR, plan: &OptimizationPlan) -> Vec<String> {
        eprintln!("\n=== Optimization Pass ===");

        if plan.constant_folding {
            Self::constant_folding(program);
        }
        if plan.common_subexpression_elimination {
            Self::common_subexpression_elimination(program);
        }
        if plan.peephole {
            Self::peephole(program);
        }
        if plan.loop_invariant_code_motion {
            Self::loop_invariant_code_motion(program);
        }
        if plan.strength_reduction {
            Self::strength_reduction(program);
        }
        if plan.inline_function {
            Self::inline_functions(program);
        }
        if plan.vectorize {
            Self::vectorize_loops(program);
        }
        if plan.loop_unrolling {
            Self::loop_unrolling(program);
        }
        if plan.dead_code_elimination {
            Self::dead_code_elimination(program);
        }

        plan.enabled_passes()
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
                                eprintln!("[Constant Fold] {} {} {} -> {}", a, op_str, b, result);
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
                                eprintln!("[Constant Fold] {} {} {} -> {}", a, op_str, b, result);
                                Some(result)
                            } else {
                                None
                            }
                        }
                        OpCode::Neg => {
                            let v = Self::get_const(&instr.operands[0], &constants);
                            if let Some(a) = v {
                                eprintln!("[Constant Fold] -{} -> {}", a, -a);
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

    /// Optimization: Local common subexpression elimination (value numbering)
    fn common_subexpression_elimination(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut versions: HashMap<String, u64> = HashMap::new();
                let mut expr_map: HashMap<ExprKey, ExprValue> = HashMap::new();

                for instr in &mut block.instructions {
                    if Self::is_control_flow(&instr.opcode) {
                        expr_map.clear();
                        versions.clear();
                        continue;
                    }

                    if Self::is_pure_op(&instr.opcode) {
                        if let Some(result_name) = instr.result.clone() {
                            if let Some(key) = Self::build_expr_key(&instr.opcode, &instr.operands, &versions) {
                                if let Some(existing) = expr_map.get(&key) {
                                    let current_version = *versions.get(&existing.result).unwrap_or(&0);
                                    if current_version == existing.version {
                                        eprintln!("[CSE] Reused {} for {:?}", existing.result, instr.opcode);
                                        instr.opcode = OpCode::Copy;
                                        instr.operands = vec![existing.result.clone()];
                                        instr.intents.clear();
                                    }
                                }

                                let new_version = Self::bump_version(&result_name, &mut versions);
                                if instr.opcode != OpCode::Copy {
                                    expr_map.insert(
                                        key,
                                        ExprValue {
                                            result: result_name,
                                            version: new_version,
                                        },
                                    );
                                }
                                continue;
                            }
                        }
                    }

                    if let Some(result) = instr.result.clone() {
                        Self::bump_version(&result, &mut versions);
                    }
                }
            }
        }
    }

    /// Optimization: Peephole simplifications
    fn peephole(program: &mut ProgramIR) {
        for func in &mut program.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instructions {
                    match instr.opcode {
                        OpCode::Add => {
                            if Self::is_zero(&instr.operands[0]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[1].clone()];
                            } else if Self::is_zero(&instr.operands[1]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[0].clone()];
                            }
                        }
                        OpCode::Sub => {
                            if Self::is_zero(&instr.operands[1]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[0].clone()];
                            } else if Self::is_zero(&instr.operands[0]) {
                                instr.opcode = OpCode::Neg;
                                instr.operands = vec![instr.operands[1].clone()];
                            }
                        }
                        OpCode::Mul => {
                            if Self::is_one(&instr.operands[0]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[1].clone()];
                            } else if Self::is_one(&instr.operands[1]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[0].clone()];
                            } else if Self::is_zero(&instr.operands[0]) || Self::is_zero(&instr.operands[1]) {
                                instr.opcode = OpCode::LoadConst;
                                instr.operands = vec!["0".to_string()];
                            }
                        }
                        OpCode::Div => {
                            if Self::is_one(&instr.operands[1]) {
                                instr.opcode = OpCode::Copy;
                                instr.operands = vec![instr.operands[0].clone()];
                            }
                        }
                        OpCode::Mod => {
                            if Self::is_one(&instr.operands[1]) {
                                instr.opcode = OpCode::LoadConst;
                                instr.operands = vec!["0".to_string()];
                            }
                        }
                        OpCode::CmpEq
                        | OpCode::CmpNe
                        | OpCode::CmpLt
                        | OpCode::CmpLe
                        | OpCode::CmpGt
                        | OpCode::CmpGe => {
                            if instr.operands[0] == instr.operands[1] {
                                let result = match instr.opcode {
                                    OpCode::CmpEq => 1,
                                    OpCode::CmpNe => 0,
                                    OpCode::CmpLt => 0,
                                    OpCode::CmpLe => 1,
                                    OpCode::CmpGt => 0,
                                    OpCode::CmpGe => 1,
                                    _ => 0,
                                };
                                instr.opcode = OpCode::LoadConst;
                                instr.operands = vec![result.to_string()];
                            }
                        }
                        OpCode::Copy => {
                            if instr.result.as_ref() == instr.operands.first() {
                                instr.opcode = OpCode::Nop;
                                instr.operands.clear();
                                instr.result = None;
                            }
                        }
                        _ => {}
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
                        eprintln!("[DCE] Removed unreachable: {:?}", instr.opcode);
                        return false;
                    }

                    if instr.opcode == OpCode::Return {
                        seen_return = true;
                        return true;
                    }

                    if instr.opcode == OpCode::StoreVar {
                        let store_target = instr.result.as_ref();
                        let store_is_needed = store_target
                            .map(|r| used_vars.contains(r) || loop_vars.contains(r))
                            .unwrap_or(false);

                        if !store_is_needed {
                            eprintln!("[DCE] Removed dead store: {:?}", instr.result);
                            return false;
                        }

                        return true;
                    }

                    // Instructions with side effects
                    let has_side_effect = matches!(
                        instr.opcode,
                        OpCode::Return | OpCode::Call | OpCode::Jump | OpCode::Branch | OpCode::Label
                    );
                    
                    let is_used = instr.result.as_ref()
                        .map(|r| used_vars.contains(r))
                        .unwrap_or(false);

                    if !has_side_effect && !is_used {
                        eprintln!("[DCE] Removed dead code: {:?} -> {:?}", instr.opcode, instr.result);
                        return false;
                    }

                    true
                });

                let removed = original_count - block.instructions.len();
                if removed > 0 {
                    eprintln!("[DCE] Eliminated {} dead instruction(s)", removed);
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
                                    eprintln!("[LICM] Hoisted {:?} out of loop", hoisted.opcode);
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
                                    eprintln!("[Strength Reduction] {} * 2 -> {} + {}", 
                                        instr.operands[0], instr.operands[0], instr.operands[0]);
                                    instr.opcode = OpCode::Add;
                                    let op = instr.operands[0].clone();
                                    instr.operands = vec![op.clone(), op];
                                } else if instr.operands[0] == "2" {
                                    eprintln!("[Strength Reduction] 2 * {} -> {} + {}", 
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
                                eprintln!("[Strength Reduction] {} / 1 -> copy", instr.operands[0]);
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

    /// Loop unrolling (limited to small constant trip-count for-loops)
    fn loop_unrolling(program: &mut ProgramIR) {
        const MAX_UNROLL: i64 = 8;

        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut idx = 0usize;
                let mut temp_counter = Self::next_unroll_temp_seed(block);

                while idx < block.instructions.len() {
                    let label = match block.instructions[idx].opcode {
                        OpCode::Label => block.instructions[idx].operands.first().cloned(),
                        _ => None,
                    };

                    let Some(label) = label else {
                        idx += 1;
                        continue;
                    };

                    let plan = if label.contains("for_start") {
                        Self::plan_for_loop_unroll(block, idx)
                    } else if label.contains("while_start") {
                        Self::plan_while_loop_unroll(block, idx)
                    } else {
                        None
                    };

                    let Some(plan) = plan else {
                        idx += 1;
                        continue;
                    };

                    if plan.trip_count <= 0 || plan.trip_count > MAX_UNROLL {
                        idx += 1;
                        continue;
                    }

                    Self::apply_unroll(block, &plan, &mut temp_counter);
                    eprintln!("[Unroll] Unrolled loop {} ({} iterations)", plan.loop_var, plan.trip_count);
                    idx = 0;
                }
            }
        }
    }

    /// Inline simple calls when the callee is a straight-line single block
    fn inline_functions(program: &mut ProgramIR) {
        let mut function_map: HashMap<String, FunctionIR> = HashMap::new();
        for func in &program.functions {
            function_map.insert(func.name.clone(), func.clone());
        }

        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut new_instructions = Vec::new();
                let mut temp_counter = Self::next_unroll_temp_seed(block);

                for instr in &block.instructions {
                    if instr.opcode == OpCode::Call {
                        if let Some(inlined) = Self::inline_call(instr, &function_map, &mut temp_counter) {
                            new_instructions.extend(inlined);
                            continue;
                        }
                    }
                    new_instructions.push(instr.clone());
                }

                block.instructions = new_instructions;
            }
        }
    }

    /// Vectorization (pairwise loop body replication with stride update)
    fn vectorize_loops(program: &mut ProgramIR) {
        const VECTOR_FACTOR: i64 = 2;

        for func in &mut program.functions {
            for block in &mut func.blocks {
                let mut idx = 0usize;
                let mut temp_counter = Self::next_unroll_temp_seed(block);
                let mut vec_counter = 0usize;

                while idx < block.instructions.len() {
                    let label = match block.instructions[idx].opcode {
                        OpCode::Label => block.instructions[idx].operands.first().cloned(),
                        _ => None,
                    };

                    let Some(label) = label else {
                        idx += 1;
                        continue;
                    };

                    if !label.contains("for_start") {
                        idx += 1;
                        continue;
                    }

                    let Some(loop_info) = Self::analyze_for_loop(block, idx) else {
                        idx += 1;
                        continue;
                    };

                    let trip_count = Self::compute_trip_count(
                        loop_info.start_value,
                        loop_info.end_value,
                        loop_info.step_value,
                        &loop_info.cmp_opcode,
                    );

                    let Some(trip_count) = trip_count else {
                        idx += 1;
                        continue;
                    };

                    if trip_count <= 1
                        || trip_count % VECTOR_FACTOR != 0
                        || loop_info.step_value <= 0
                    {
                        idx += 1;
                        continue;
                    }

                    if Self::loop_already_vectorized(
                        block,
                        &loop_info.loop_var,
                        loop_info.body_end + 1,
                        loop_info.increment_add_idx,
                    ) {
                        idx += 1;
                        continue;
                    }

                    let vec_var = Self::next_vector_var(&loop_info.loop_var, &mut vec_counter);
                    let mut new_instructions = Vec::new();

                    for i in 0..block.instructions.len() {
                        if i == loop_info.increment_add_idx {
                            let tmp = Self::next_unroll_temp(&mut temp_counter);
                            new_instructions.push(Instruction {
                                opcode: OpCode::Add,
                                operands: vec![
                                    loop_info.loop_var.clone(),
                                    loop_info.step_value.to_string(),
                                ],
                                result: Some(tmp.clone()),
                                intents: vec![],
                                profile: ProfileData::new(),
                            });
                            new_instructions.push(Instruction {
                                opcode: OpCode::StoreVar,
                                operands: vec![tmp],
                                result: Some(vec_var.clone()),
                                intents: vec![],
                                profile: ProfileData::new(),
                            });

                            let mut temp_map: HashMap<String, String> = HashMap::new();
                            for instr in &block.instructions[loop_info.body_start..=loop_info.body_end] {
                                let renamed = Self::rename_temps(instr, &mut temp_map, &mut temp_counter);
                                let substituted = Self::substitute_var(&renamed, &loop_info.loop_var, &vec_var);
                                new_instructions.push(substituted);
                            }
                        }

                        let mut instr = block.instructions[i].clone();
                        if i == loop_info.increment_add_idx {
                            Self::update_increment_step(&mut instr, &loop_info.loop_var, loop_info.step_value * VECTOR_FACTOR);
                        }
                        new_instructions.push(instr);
                    }

                    eprintln!("[Vectorize] Vectorized loop {} (factor {})", loop_info.loop_var, VECTOR_FACTOR);
                    block.instructions = new_instructions;
                    idx = 0;
                }
            }
        }
    }

    fn is_control_flow(opcode: &OpCode) -> bool {
        matches!(opcode, OpCode::Label | OpCode::Jump | OpCode::Branch | OpCode::Return | OpCode::Call)
    }

    fn is_pure_op(opcode: &OpCode) -> bool {
        matches!(
            opcode,
            OpCode::Add
                | OpCode::Sub
                | OpCode::Mul
                | OpCode::Div
                | OpCode::Mod
                | OpCode::Neg
                | OpCode::CmpEq
                | OpCode::CmpNe
                | OpCode::CmpLt
                | OpCode::CmpLe
                | OpCode::CmpGt
                | OpCode::CmpGe
        )
    }

    fn build_expr_key(
        opcode: &OpCode,
        operands: &[String],
        versions: &HashMap<String, u64>,
    ) -> Option<ExprKey> {
        if !Self::is_pure_op(opcode) {
            return None;
        }

        match opcode {
            OpCode::Neg => {
                let op1 = Self::operand_key(operands.get(0)?, versions);
                Some(ExprKey {
                    opcode: opcode.clone(),
                    op1,
                    op2: None,
                })
            }
            _ => {
                let op1 = Self::operand_key(operands.get(0)?, versions);
                let op2 = Self::operand_key(operands.get(1)?, versions);
                if Self::is_commutative(opcode) {
                    let (left, right) = Self::sort_operands(op1, op2);
                    Some(ExprKey {
                        opcode: opcode.clone(),
                        op1: left,
                        op2: Some(right),
                    })
                } else {
                    Some(ExprKey {
                        opcode: opcode.clone(),
                        op1,
                        op2: Some(op2),
                    })
                }
            }
        }
    }

    fn operand_key(operand: &str, versions: &HashMap<String, u64>) -> OperandKey {
        if operand.parse::<i64>().is_ok() {
            OperandKey::Const(operand.to_string())
        } else {
            let version = *versions.get(operand).unwrap_or(&0);
            OperandKey::Var(operand.to_string(), version)
        }
    }

    fn is_commutative(opcode: &OpCode) -> bool {
        matches!(opcode, OpCode::Add | OpCode::Mul | OpCode::CmpEq | OpCode::CmpNe)
    }

    fn sort_operands(a: OperandKey, b: OperandKey) -> (OperandKey, OperandKey) {
        let key_a = Self::operand_sort_key(&a);
        let key_b = Self::operand_sort_key(&b);
        if key_a <= key_b {
            (a, b)
        } else {
            (b, a)
        }
    }

    fn operand_sort_key(key: &OperandKey) -> String {
        match key {
            OperandKey::Const(val) => format!("c:{}", val),
            OperandKey::Var(name, ver) => format!("v:{}:{}", name, ver),
        }
    }

    fn bump_version(name: &str, versions: &mut HashMap<String, u64>) -> u64 {
        let entry = versions.entry(name.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    }

    fn is_zero(value: &str) -> bool {
        value.parse::<i64>().map(|v| v == 0).unwrap_or(false)
    }

    fn is_one(value: &str) -> bool {
        value.parse::<i64>().map(|v| v == 1).unwrap_or(false)
    }

    fn next_unroll_temp_seed(block: &BasicBlock) -> usize {
        let mut max = 0usize;
        for instr in &block.instructions {
            for op in &instr.operands {
                if let Some(idx) = op.strip_prefix("_unroll_tmp_") {
                    if let Ok(val) = idx.parse::<usize>() {
                        max = max.max(val + 1);
                    }
                }
            }
            if let Some(res) = &instr.result {
                if let Some(idx) = res.strip_prefix("_unroll_tmp_") {
                    if let Ok(val) = idx.parse::<usize>() {
                        max = max.max(val + 1);
                    }
                }
            }
        }
        max
    }

    fn next_unroll_temp(counter: &mut usize) -> String {
        let name = format!("_unroll_tmp_{}", *counter);
        *counter += 1;
        name
    }

    fn is_temp_name(name: &str) -> bool {
        let mut chars = name.chars();
        let Some('t') = chars.next() else { return false; };
        chars.all(|c| c.is_ascii_digit())
    }

    fn rename_temps(
        instr: &Instruction,
        temp_map: &mut HashMap<String, String>,
        counter: &mut usize,
    ) -> Instruction {
        let mut cloned = instr.clone();
        cloned.operands = instr
            .operands
            .iter()
            .map(|op| Self::rename_temp(op, temp_map, counter))
            .collect();

        if let Some(result) = &instr.result {
            if Self::is_temp_name(result) {
                let renamed = Self::rename_temp(result, temp_map, counter);
                cloned.result = Some(renamed);
            }
        }

        cloned
    }

    fn rename_temp(
        name: &str,
        temp_map: &mut HashMap<String, String>,
        counter: &mut usize,
    ) -> String {
        if !Self::is_temp_name(name) {
            return name.to_string();
        }

        temp_map
            .entry(name.to_string())
            .or_insert_with(|| Self::next_unroll_temp(counter))
            .clone()
    }

    fn analyze_for_loop(block: &BasicBlock, start_idx: usize) -> Option<ForLoopInfo> {
        let label = block.instructions[start_idx].operands.first()?.clone();
        let end_label = label.replace("_start", "_end");

        let mut end_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().skip(start_idx + 1) {
            if instr.opcode == OpCode::Label && instr.operands.first() == Some(&end_label) {
                end_idx = Some(idx);
                break;
            }
        }
        let end_idx = end_idx?;

        let mut branch_idx = None;
        let mut cmp_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().take(end_idx).skip(start_idx + 1) {
            if instr.opcode == OpCode::CmpLt || instr.opcode == OpCode::CmpLe {
                cmp_idx = Some(idx);
            }
            if instr.opcode == OpCode::Branch && instr.operands.get(1) == Some(&end_label) {
                branch_idx = Some(idx);
                break;
            }
        }

        let cmp_idx = cmp_idx?;
        let branch_idx = branch_idx?;
        let cmp_opcode = block.instructions[cmp_idx].opcode.clone();
        let loop_var = block.instructions[cmp_idx].operands.get(0)?.clone();
        let end_var = block.instructions[cmp_idx].operands.get(1)?.clone();

        let mut jump_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().take(end_idx).skip(branch_idx + 1) {
            if instr.opcode == OpCode::Jump && instr.operands.first() == Some(&label) {
                jump_idx = Some(idx);
                break;
            }
        }
        let jump_idx = jump_idx?;

        let mut increment_add_idx = None;
        let mut _increment_store_idx = None;
        for idx in (branch_idx + 1..jump_idx).rev() {
            let instr = &block.instructions[idx];
            if instr.opcode == OpCode::StoreVar && instr.result.as_deref() == Some(&loop_var) {
                _increment_store_idx = Some(idx);
                if idx >= 1 {
                    let add_instr = &block.instructions[idx - 1];
                    if matches!(add_instr.opcode, OpCode::Add | OpCode::Sub)
                        && add_instr.operands.contains(&loop_var)
                    {
                        increment_add_idx = Some(idx - 1);
                    }
                }
                break;
            }
        }
        let increment_add_idx = increment_add_idx?;

        let body_start = branch_idx + 1;
        if increment_add_idx <= body_start {
            return None;
        }
        let body_end = increment_add_idx - 1;

        for instr in &block.instructions[body_start..=body_end] {
            if matches!(instr.opcode, OpCode::Label | OpCode::Jump | OpCode::Branch) {
                return None;
            }
            if instr.opcode == OpCode::StoreVar && instr.result.as_deref() == Some(&loop_var) {
                return None;
            }
        }

        let const_map = Self::collect_constants(&block.instructions[..start_idx]);
        let start_value = Self::find_last_store_const(&block.instructions[..start_idx], &loop_var, &const_map)?;
        let end_value = Self::find_last_store_const(&block.instructions[..start_idx], &end_var, &const_map)?;

        let step_const_map = Self::collect_constants(&block.instructions[..increment_add_idx]);
        let step_value = Self::step_from_increment(
            &block.instructions[increment_add_idx],
            &loop_var,
            &step_const_map,
        )?;

        Some(ForLoopInfo {
            start_idx,
            end_idx,
            body_start,
            body_end,
            loop_var,
            start_value,
            end_value,
            cmp_opcode,
            step_value,
            increment_add_idx,
        })
    }

    fn collect_constants(instructions: &[Instruction]) -> HashMap<String, i64> {
        let mut constants = HashMap::new();
        for instr in instructions {
            match instr.opcode {
                OpCode::LoadConst => {
                    if let Some(res) = &instr.result {
                        if let Ok(val) = instr.operands[0].parse::<i64>() {
                            constants.insert(res.clone(), val);
                        }
                    }
                }
                OpCode::Copy | OpCode::StoreVar | OpCode::LoadVar => {
                    if let Some(res) = &instr.result {
                        if let Some(value) = Self::resolve_const(&instr.operands[0], &constants) {
                            constants.insert(res.clone(), value);
                        } else {
                            constants.remove(res);
                        }
                    }
                }
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => {
                    if let Some(res) = &instr.result {
                        let a = Self::resolve_const(&instr.operands[0], &constants);
                        let b = Self::resolve_const(&instr.operands[1], &constants);
                        if let (Some(a), Some(b)) = (a, b) {
                            let value = match instr.opcode {
                                OpCode::Add => a + b,
                                OpCode::Sub => a - b,
                                OpCode::Mul => a * b,
                                OpCode::Div => if b != 0 { a / b } else { 0 },
                                OpCode::Mod => if b != 0 { a % b } else { 0 },
                                _ => 0,
                            };
                            constants.insert(res.clone(), value);
                        } else {
                            constants.remove(res);
                        }
                    }
                }
                OpCode::Neg => {
                    if let Some(res) = &instr.result {
                        if let Some(val) = Self::resolve_const(&instr.operands[0], &constants) {
                            constants.insert(res.clone(), -val);
                        } else {
                            constants.remove(res);
                        }
                    }
                }
                _ => {
                    if let Some(res) = &instr.result {
                        constants.remove(res);
                    }
                }
            }
        }
        constants
    }

    fn resolve_const(operand: &str, constants: &HashMap<String, i64>) -> Option<i64> {
        if let Ok(value) = operand.parse::<i64>() {
            return Some(value);
        }
        constants.get(operand).cloned()
    }

    fn find_last_store_const(
        instructions: &[Instruction],
        var_name: &str,
        constants: &HashMap<String, i64>,
    ) -> Option<i64> {
        for instr in instructions.iter().rev() {
            if instr.opcode == OpCode::StoreVar && instr.result.as_deref() == Some(var_name) {
                return Self::resolve_const(&instr.operands[0], constants);
            }
        }
        None
    }

    fn step_from_increment(
        instr: &Instruction,
        loop_var: &str,
        constants: &HashMap<String, i64>,
    ) -> Option<i64> {
        if !matches!(instr.opcode, OpCode::Add | OpCode::Sub) {
            return None;
        }

        if !instr.operands.contains(&loop_var.to_string()) {
            return None;
        }

        let other = instr
            .operands
            .iter()
            .find(|op| *op != loop_var)
            .cloned()?;
        let step = Self::resolve_const(&other, constants)?;
        if step == 0 {
            return None;
        }

        match instr.opcode {
            OpCode::Add => Some(step),
            OpCode::Sub => Some(-step),
            _ => None,
        }
    }

    fn compute_trip_count(
        start: i64,
        end: i64,
        step: i64,
        cmp_opcode: &OpCode,
    ) -> Option<i64> {
        if step == 0 {
            return None;
        }

        match cmp_opcode {
            OpCode::CmpLt => {
                if step <= 0 {
                    return None;
                }
                if start >= end {
                    return Some(0);
                }
                Some((end - start + step - 1) / step)
            }
            OpCode::CmpLe => {
                if step <= 0 {
                    return None;
                }
                if start > end {
                    return Some(0);
                }
                Some(((end - start) / step) + 1)
            }
            OpCode::CmpGt => {
                if step >= 0 {
                    return None;
                }
                if start <= end {
                    return Some(0);
                }
                let step_abs = -step;
                Some((start - end + step_abs - 1) / step_abs)
            }
            OpCode::CmpGe => {
                if step >= 0 {
                    return None;
                }
                if start < end {
                    return Some(0);
                }
                let step_abs = -step;
                Some(((start - end) / step_abs) + 1)
            }
            _ => None,
        }
    }

    fn plan_for_loop_unroll(block: &BasicBlock, start_idx: usize) -> Option<LoopUnrollPlan> {
        let info = Self::analyze_for_loop(block, start_idx)?;
        let trip_count = Self::compute_trip_count(
            info.start_value,
            info.end_value,
            info.step_value,
            &info.cmp_opcode,
        )?;

        Some(LoopUnrollPlan {
            start_idx: info.start_idx,
            end_idx: info.end_idx,
            body_start: info.body_start,
            body_end: info.body_end,
            loop_var: info.loop_var,
            start_value: info.start_value,
            step_value: info.step_value,
            trip_count,
        })
    }

    fn plan_while_loop_unroll(block: &BasicBlock, start_idx: usize) -> Option<LoopUnrollPlan> {
        let info = Self::analyze_while_loop(block, start_idx)?;
        let trip_count = Self::compute_trip_count(
            info.start_value,
            info.bound_value,
            info.step_value,
            &info.cmp_opcode,
        )?;

        Some(LoopUnrollPlan {
            start_idx: info.start_idx,
            end_idx: info.end_idx,
            body_start: info.body_start,
            body_end: info.body_end,
            loop_var: info.loop_var,
            start_value: info.start_value,
            step_value: info.step_value,
            trip_count,
        })
    }

    fn apply_unroll(block: &mut BasicBlock, plan: &LoopUnrollPlan, temp_counter: &mut usize) {
        let mut new_instructions = Vec::new();
        new_instructions.extend_from_slice(&block.instructions[..plan.start_idx]);

        for iter in 0..plan.trip_count {
            let iter_value = plan.start_value + (plan.step_value * iter);
            let tmp = Self::next_unroll_temp(temp_counter);
            new_instructions.push(Instruction {
                opcode: OpCode::LoadConst,
                operands: vec![iter_value.to_string()],
                result: Some(tmp.clone()),
                intents: vec![],
                profile: ProfileData::new(),
            });
            new_instructions.push(Instruction {
                opcode: OpCode::StoreVar,
                operands: vec![tmp],
                result: Some(plan.loop_var.clone()),
                intents: vec![],
                profile: ProfileData::new(),
            });

            let mut temp_map: HashMap<String, String> = HashMap::new();
            for instr in &block.instructions[plan.body_start..=plan.body_end] {
                let renamed = Self::rename_temps(instr, &mut temp_map, temp_counter);
                new_instructions.push(renamed);
            }
        }

        let final_value = plan.start_value + (plan.step_value * plan.trip_count);
        let tmp = Self::next_unroll_temp(temp_counter);
        new_instructions.push(Instruction {
            opcode: OpCode::LoadConst,
            operands: vec![final_value.to_string()],
            result: Some(tmp.clone()),
            intents: vec![],
            profile: ProfileData::new(),
        });
        new_instructions.push(Instruction {
            opcode: OpCode::StoreVar,
            operands: vec![tmp],
            result: Some(plan.loop_var.clone()),
            intents: vec![],
            profile: ProfileData::new(),
        });

        if plan.end_idx + 1 < block.instructions.len() {
            new_instructions.extend_from_slice(&block.instructions[plan.end_idx + 1..]);
        }

        block.instructions = new_instructions;
    }

    fn analyze_while_loop(block: &BasicBlock, start_idx: usize) -> Option<WhileLoopInfo> {
        let label = block.instructions[start_idx].operands.first()?.clone();
        let end_label = label.replace("_start", "_end");

        let mut end_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().skip(start_idx + 1) {
            if instr.opcode == OpCode::Label && instr.operands.first() == Some(&end_label) {
                end_idx = Some(idx);
                break;
            }
        }
        let end_idx = end_idx?;

        let mut branch_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().take(end_idx).skip(start_idx + 1) {
            if instr.opcode == OpCode::Branch && instr.operands.get(1) == Some(&end_label) {
                branch_idx = Some(idx);
                break;
            }
        }
        let branch_idx = branch_idx?;

        let branch_cond = block.instructions[branch_idx].operands.get(0)?.clone();
        let mut cmp_eq_idx = None;
        for idx in (start_idx + 1..branch_idx).rev() {
            let instr = &block.instructions[idx];
            if instr.opcode == OpCode::CmpEq
                && instr.result.as_deref() == Some(&branch_cond)
                && instr.operands.get(1).map(|s| s == "0").unwrap_or(false)
            {
                cmp_eq_idx = Some(idx);
                break;
            }
        }
        let cmp_eq_idx = cmp_eq_idx?;

        let cond_temp = block.instructions[cmp_eq_idx].operands.get(0)?.clone();
        let mut cmp_idx = None;
        for idx in (start_idx + 1..cmp_eq_idx).rev() {
            let instr = &block.instructions[idx];
            if instr.result.as_deref() == Some(&cond_temp)
                && matches!(
                    instr.opcode,
                    OpCode::CmpLt | OpCode::CmpLe | OpCode::CmpGt | OpCode::CmpGe
                )
            {
                cmp_idx = Some(idx);
                break;
            }
        }
        let cmp_idx = cmp_idx?;
        let cmp_opcode = block.instructions[cmp_idx].opcode.clone();
        let loop_var = block.instructions[cmp_idx].operands.get(0)?.clone();
        let bound_op = block.instructions[cmp_idx].operands.get(1)?.clone();

        let mut jump_idx = None;
        for (idx, instr) in block.instructions.iter().enumerate().take(end_idx).skip(branch_idx + 1) {
            if instr.opcode == OpCode::Jump && instr.operands.first() == Some(&label) {
                jump_idx = Some(idx);
                break;
            }
        }
        let jump_idx = jump_idx?;

        let mut increment_add_idx = None;
        let mut _increment_store_idx = None;
        for idx in (branch_idx + 1..jump_idx).rev() {
            let instr = &block.instructions[idx];
            if instr.opcode == OpCode::StoreVar && instr.result.as_deref() == Some(&loop_var) {
                _increment_store_idx = Some(idx);
                if idx >= 1 {
                    let add_instr = &block.instructions[idx - 1];
                    if matches!(add_instr.opcode, OpCode::Add | OpCode::Sub)
                        && add_instr.operands.contains(&loop_var)
                    {
                        increment_add_idx = Some(idx - 1);
                    }
                }
                break;
            }
        }
        let increment_add_idx = increment_add_idx?;

        let body_start = branch_idx + 1;
        if increment_add_idx <= body_start {
            return None;
        }
        let body_end = increment_add_idx - 1;

        for instr in &block.instructions[body_start..=body_end] {
            if matches!(instr.opcode, OpCode::Label | OpCode::Jump | OpCode::Branch) {
                return None;
            }
            if instr.opcode == OpCode::StoreVar && instr.result.as_deref() == Some(&loop_var) {
                return None;
            }
        }

        let const_map = Self::collect_constants(&block.instructions[..start_idx]);
        let start_value = Self::find_last_store_const(&block.instructions[..start_idx], &loop_var, &const_map)?;
        let bound_value = Self::resolve_const(&bound_op, &const_map)?;

        let step_const_map = Self::collect_constants(&block.instructions[..increment_add_idx]);
        let step_value = Self::step_from_increment(
            &block.instructions[increment_add_idx],
            &loop_var,
            &step_const_map,
        )?;

        Some(WhileLoopInfo {
            start_idx,
            end_idx,
            body_start,
            body_end,
            loop_var,
            start_value,
            bound_value,
            cmp_opcode,
            step_value,
        })
    }

    fn substitute_var(instr: &Instruction, from: &str, to: &str) -> Instruction {
        let mut cloned = instr.clone();
        cloned.operands = instr
            .operands
            .iter()
            .map(|op| if op == from { to.to_string() } else { op.clone() })
            .collect();
        cloned
    }

    fn update_increment_step(instr: &mut Instruction, loop_var: &str, new_step: i64) {
        let step_abs = new_step.abs();
        instr.opcode = if new_step >= 0 { OpCode::Add } else { OpCode::Sub };
        instr.operands = vec![loop_var.to_string(), step_abs.to_string()];
    }

    fn next_vector_var(base: &str, counter: &mut usize) -> String {
        let name = format!("{}_vec_{}", base, *counter);
        *counter += 1;
        name
    }

    fn loop_already_vectorized(
        block: &BasicBlock,
        loop_var: &str,
        start_idx: usize,
        end_idx: usize,
    ) -> bool {
        if start_idx >= end_idx || end_idx > block.instructions.len() {
            return false;
        }

        let prefix = format!("{}_vec_", loop_var);
        block.instructions[start_idx..end_idx]
            .iter()
            .any(|instr| instr.opcode == OpCode::StoreVar
                && instr.result.as_deref().map(|r| r.starts_with(&prefix)).unwrap_or(false))
    }

    fn inline_call(
        instr: &Instruction,
        function_map: &HashMap<String, FunctionIR>,
        temp_counter: &mut usize,
    ) -> Option<Vec<Instruction>> {
        let callee_name = instr.operands.first()?.clone();
        let callee = function_map.get(&callee_name)?;

        if callee.blocks.len() != 1 {
            return None;
        }

        let block = &callee.blocks[0];
        if block.instructions.iter().any(|i| matches!(i.opcode, OpCode::Label | OpCode::Jump | OpCode::Branch)) {
            return None;
        }

        let mut param_map = HashMap::new();
        if callee.params.len() + 1 != instr.operands.len() {
            return None;
        }

        for (param, arg) in callee.params.iter().zip(instr.operands.iter().skip(1)) {
            param_map.insert(param.clone(), arg.clone());
        }

        for instr in &block.instructions {
            if instr.opcode == OpCode::StoreVar {
                if let Some(res) = &instr.result {
                    if !Self::is_temp_name(res) && !param_map.contains_key(res) {
                        return None;
                    }
                }
            }
        }

        let mut inlined = Vec::new();
        let mut temp_map: HashMap<String, String> = HashMap::new();

        for callee_instr in &block.instructions {
            if callee_instr.opcode == OpCode::Return {
                if let Some(result) = instr.result.clone() {
                    let operand = callee_instr.operands.first().cloned()?;
                    let mapped = param_map.get(&operand).cloned().unwrap_or(operand);
                    let operand = Self::rename_temp(&mapped, &mut temp_map, temp_counter);
                    inlined.push(Instruction {
                        opcode: OpCode::Copy,
                        operands: vec![operand],
                        result: Some(result),
                        intents: vec![],
                        profile: ProfileData::new(),
                    });
                }
                continue;
            }

            let mut cloned = Self::rename_temps(callee_instr, &mut temp_map, temp_counter);
            cloned.operands = cloned
                .operands
                .into_iter()
                .map(|op| param_map.get(&op).cloned().unwrap_or(op))
                .collect();
            if let Some(res) = cloned.result.clone() {
                if let Some(mapped) = param_map.get(&res) {
                    cloned.result = Some(mapped.clone());
                }
            }
            inlined.push(cloned);
        }

        Some(inlined)
    }
}

#[derive(Debug, Clone)]
struct ForLoopInfo {
    start_idx: usize,
    end_idx: usize,
    body_start: usize,
    body_end: usize,
    loop_var: String,
    start_value: i64,
    end_value: i64,
    cmp_opcode: OpCode,
    step_value: i64,
    increment_add_idx: usize,
}

#[derive(Debug, Clone)]
struct WhileLoopInfo {
    start_idx: usize,
    end_idx: usize,
    body_start: usize,
    body_end: usize,
    loop_var: String,
    start_value: i64,
    bound_value: i64,
    cmp_opcode: OpCode,
    step_value: i64,
}

#[derive(Debug, Clone)]
struct LoopUnrollPlan {
    start_idx: usize,
    end_idx: usize,
    body_start: usize,
    body_end: usize,
    loop_var: String,
    start_value: i64,
    step_value: i64,
    trip_count: i64,
}

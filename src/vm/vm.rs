use std::collections::HashMap;
use std::time::Instant;
use crate::ir::ir::*;

const HOT_THRESHOLD: u64 = 3;  // instructions executed more than this are "hot"

pub struct NyxVM {
    pub variables: HashMap<String, i64>,
    call_stack: Vec<HashMap<String, i64>>,
}

impl NyxVM {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn run_program(&mut self, program: &mut ProgramIR) {
        self.variables.clear();
        self.call_stack.clear();

        let func_map = Self::build_function_map(program);
        if let Some(&main_idx) = func_map.get("main") {
            self.run_function_by_index(program, main_idx, &func_map, true);
        } else {
            for idx in 0..program.functions.len() {
                self.run_function_by_index(program, idx, &func_map, true);
            }
        }

        // Mark hot instructions and print detailed profiling
        self.print_profile(program);
    }

    fn print_profile(&self, program: &mut ProgramIR) {
        eprintln!("\n=== Profiling Data ===");
        
        let mut total_instructions = 0u64;
        let mut total_time_ns = 0u64;
        let mut hot_count = 0;

        for func in &mut program.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instructions {
                    if instr.profile.exec_count > 0 {
                        total_instructions += instr.profile.exec_count;
                        total_time_ns += instr.profile.total_time_ns;
                        
                        // Mark as hot if above threshold
                        if instr.profile.exec_count > HOT_THRESHOLD {
                            instr.profile.is_hot = true;
                            hot_count += 1;
                        }

                        let hot_marker = if instr.profile.is_hot { "🔥" } else { "  " };
                        eprintln!(
                            "{} {:?}: {} execs, {}ns avg",
                            hot_marker,
                            instr.opcode,
                            instr.profile.exec_count,
                            instr.profile.avg_time_ns()
                        );
                    }
                }
            }
        }

        eprintln!("---");
        eprintln!("Total: {} instruction executions", total_instructions);
        eprintln!("Total time: {}μs", total_time_ns / 1000);
        eprintln!("Hot instructions: {} (threshold: >{})", hot_count, HOT_THRESHOLD);
    }

    fn run_function_by_index(
        &mut self,
        program: &mut ProgramIR,
        func_idx: usize,
        func_map: &HashMap<String, usize>,
        is_top_level: bool,
    ) -> Option<i64> {
        let label_map = Self::build_label_map(&program.functions[func_idx]);
        let block_len = program.functions[func_idx].blocks.len();

        for block_idx in 0..block_len {
            let mut pc: usize = 0;

            loop {
                let instr_len = program.functions[func_idx].blocks[block_idx].instructions.len();
                if pc >= instr_len {
                    break;
                }

                let start_time = Instant::now();
                let (opcode, operands, result_name) = {
                    let instr = &mut program.functions[func_idx].blocks[block_idx].instructions[pc];
                    instr.profile.exec_count += 1;
                    (
                        instr.opcode.clone(),
                        instr.operands.clone(),
                        instr.result.clone(),
                    )
                };

                let mut result_value: Option<i64> = None;
                let mut jump_target: Option<usize> = None;
                let mut return_value: Option<i64> = None;

                match opcode {
                    OpCode::Jump => {
                        let target = &operands[0];
                        if let Some(&target_pc) = label_map.get(target) {
                            jump_target = Some(target_pc);
                        }
                    }

                    OpCode::Branch => {
                        let cond_var = &operands[0];
                        let target = &operands[1];
                        let cond_val = self.get_value(cond_var);

                        if cond_val != 0 {
                            if let Some(&target_pc) = label_map.get(target) {
                                jump_target = Some(target_pc);
                            }
                        }
                    }

                    OpCode::Return => {
                        let val = self.get_value(&operands[0]);
                        if is_top_level {
                            eprintln!("Program returned: {}", val);
                        }
                        result_value = Some(val);
                        return_value = Some(val);
                    }

                    OpCode::Call => {
                        let call_result = self.execute_call(program, func_map, &operands);
                        let value = call_result.unwrap_or(0);
                        if let Some(name) = result_name {
                            self.variables.insert(name, value);
                        }
                        result_value = Some(value);
                    }

                    _ => {
                        let instr = &mut program.functions[func_idx].blocks[block_idx].instructions[pc];
                        result_value = self.execute_instruction(instr);
                    }
                }

                let elapsed = start_time.elapsed().as_nanos() as u64;
                let instr = &mut program.functions[func_idx].blocks[block_idx].instructions[pc];
                instr.profile.total_time_ns += elapsed;
                if result_value.is_some() {
                    instr.profile.last_value = result_value;
                }

                if let Some(val) = return_value {
                    return Some(val);
                }

                if let Some(target_pc) = jump_target {
                    pc = target_pc;
                    continue;
                }

                pc += 1;
            }
        }

        None
    }

    fn execute_call(
        &mut self,
        program: &mut ProgramIR,
        func_map: &HashMap<String, usize>,
        operands: &[String],
    ) -> Option<i64> {
        let name = operands.first()?.clone();
        let &callee_idx = func_map.get(&name)?;
        let args: Vec<i64> = operands.iter().skip(1).map(|op| self.get_value(op)).collect();
        let params = program.functions[callee_idx].params.clone();

        self.call_stack.push(std::mem::take(&mut self.variables));
        let mut frame = HashMap::new();
        let args_len = args.len();
        for (param, arg) in params.iter().cloned().zip(args.into_iter()) {
            frame.insert(param, arg);
        }
        if args_len < params.len() {
            for param in params.into_iter().skip(args_len) {
                frame.insert(param, 0);
            }
        }
        self.variables = frame;

        let result = self.run_function_by_index(program, callee_idx, func_map, false);
        self.variables = self.call_stack.pop().unwrap_or_default();
        result
    }

    fn build_function_map(program: &ProgramIR) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for (idx, func) in program.functions.iter().enumerate() {
            map.insert(func.name.clone(), idx);
        }
        map
    }

    fn build_label_map(func: &FunctionIR) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for block in &func.blocks {
            for (idx, instr) in block.instructions.iter().enumerate() {
                if instr.opcode == OpCode::Label {
                    if let Some(label) = instr.operands.first() {
                        map.insert(label.clone(), idx);
                    }
                }
            }
        }
        map
    }

    fn execute_instruction(&mut self, instr: &mut Instruction) -> Option<i64> {
        match instr.opcode {
            OpCode::LoadConst => {
                let val = instr.operands[0].parse::<i64>().unwrap();
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
                Some(val)
            }

            OpCode::Add => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = a + b;
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::Sub => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = a - b;
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::Mul => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = a * b;
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::Div => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if b != 0 { a / b } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::Mod => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if b != 0 { a % b } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::Neg => {
                let a = self.get_value(&instr.operands[0]);
                let result = -a;
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpEq => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a == b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpNe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a != b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpLt => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a < b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpLe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a <= b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpGt => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a > b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::CmpGe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                let result = if a >= b { 1 } else { 0 };
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), result);
                }
                Some(result)
            }

            OpCode::StoreVar => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
                Some(val)
            }

            OpCode::Copy => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
                Some(val)
            }

            OpCode::LoadVar => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
                Some(val)
            }

            // Control flow handled in run_function
            OpCode::Jump | OpCode::Branch | OpCode::Return => None,
            
            // Labels are markers, no execution
            OpCode::Label | OpCode::Nop => None,

            OpCode::Call => {
                // Function calls to be implemented later
                None
            }
        }
    }

    fn get_value(&self, name: &str) -> i64 {
        if let Ok(v) = name.parse::<i64>() {
            return v;
        }
        *self.variables.get(name).unwrap_or(&0)
    }
}

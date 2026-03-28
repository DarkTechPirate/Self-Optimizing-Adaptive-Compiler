use std::collections::HashMap;
use std::time::Instant;
use crate::ir::ir::*;

const HOT_THRESHOLD: u64 = 3;  // instructions executed more than this are "hot"

pub struct NyxVM {
    pub variables: HashMap<String, i64>,
}

impl NyxVM {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn run_program(&mut self, program: &mut ProgramIR) {
        self.variables.clear();

        for func in &mut program.functions {
            self.run_function(func);
        }

        // Mark hot instructions and print detailed profiling
        self.print_profile(program);
    }

    fn print_profile(&self, program: &mut ProgramIR) {
        println!("\n=== Profiling Data ===");
        
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
                        println!(
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

        println!("---");
        println!("Total: {} instruction executions", total_instructions);
        println!("Total time: {}μs", total_time_ns / 1000);
        println!("Hot instructions: {} (threshold: >{})", hot_count, HOT_THRESHOLD);
    }

    fn run_function(&mut self, func: &mut FunctionIR) {
        // Build label index for all blocks
        let mut label_map: HashMap<String, usize> = HashMap::new();
        
        for block in &func.blocks {
            for (idx, instr) in block.instructions.iter().enumerate() {
                if instr.opcode == OpCode::Label {
                    if let Some(label) = instr.operands.first() {
                        label_map.insert(label.clone(), idx);
                    }
                }
            }
        }

        // Execute instructions with control flow
        for block in &mut func.blocks {
            let mut pc: usize = 0;
            
            while pc < block.instructions.len() {
                let start_time = Instant::now();
                
                // Read instruction data first
                let opcode = block.instructions[pc].opcode.clone();
                let operands = block.instructions[pc].operands.clone();
                block.instructions[pc].profile.exec_count += 1;

                match opcode {
                    OpCode::Jump => {
                        let target = &operands[0];
                        let elapsed = start_time.elapsed().as_nanos() as u64;
                        block.instructions[pc].profile.total_time_ns += elapsed;
                        if let Some(&target_pc) = label_map.get(target) {
                            pc = target_pc;
                            continue;
                        }
                    }
                    
                    OpCode::Branch => {
                        let cond_var = &operands[0];
                        let target = &operands[1];
                        let cond_val = self.get_value(cond_var);
                        let elapsed = start_time.elapsed().as_nanos() as u64;
                        block.instructions[pc].profile.total_time_ns += elapsed;
                        
                        if cond_val != 0 {
                            if let Some(&target_pc) = label_map.get(target) {
                                pc = target_pc;
                                continue;
                            }
                        }
                    }
                    
                    OpCode::Return => {
                        let val = self.get_value(&operands[0]);
                        let elapsed = start_time.elapsed().as_nanos() as u64;
                        block.instructions[pc].profile.total_time_ns += elapsed;
                        block.instructions[pc].profile.last_value = Some(val);
                        println!("Program returned: {}", val);
                        return;
                    }
                    
                    _ => {
                        let instr = &mut block.instructions[pc];
                        let result = self.execute_instruction(instr);
                        let elapsed = start_time.elapsed().as_nanos() as u64;
                        block.instructions[pc].profile.total_time_ns += elapsed;
                        block.instructions[pc].profile.last_value = result;
                    }
                }
                
                pc += 1;
            }
        }
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

use std::collections::HashMap;
use crate::ir::ir::*;

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

        println!("\n=== Profiling Data ===");

        for func in &program.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if instr.profile.exec_count > 0 {
                        println!(
                            "{:?} executed {} times",
                            instr.opcode,
                            instr.profile.exec_count
                        );
                    }
                }
            }
        }
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
                let instr = &mut block.instructions[pc];
                instr.profile.exec_count += 1;

                match &instr.opcode {
                    OpCode::Jump => {
                        let target = &instr.operands[0];
                        if let Some(&target_pc) = label_map.get(target) {
                            pc = target_pc;
                            continue;
                        }
                    }
                    
                    OpCode::Branch => {
                        let cond_var = &instr.operands[0];
                        let target = &instr.operands[1];
                        let cond_val = self.get_value(cond_var);
                        
                        if cond_val != 0 {
                            if let Some(&target_pc) = label_map.get(target) {
                                pc = target_pc;
                                continue;
                            }
                        }
                    }
                    
                    OpCode::Return => {
                        let val = self.get_value(&instr.operands[0]);
                        println!("Program returned: {}", val);
                        return;
                    }
                    
                    _ => {
                        self.execute_instruction(instr);
                    }
                }
                
                pc += 1;
            }
        }
    }

    fn execute_instruction(&mut self, instr: &mut Instruction) {
        match instr.opcode {
            OpCode::LoadConst => {
                let val = instr.operands[0].parse::<i64>().unwrap();
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
            }

            OpCode::Add => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), a + b);
                }
            }

            OpCode::Sub => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), a - b);
                }
            }

            OpCode::Mul => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), a * b);
                }
            }

            OpCode::Div => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if b != 0 { a / b } else { 0 });
                }
            }

            OpCode::Mod => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if b != 0 { a % b } else { 0 });
                }
            }

            OpCode::Neg => {
                let a = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), -a);
                }
            }

            OpCode::CmpEq => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a == b { 1 } else { 0 });
                }
            }

            OpCode::CmpNe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a != b { 1 } else { 0 });
                }
            }

            OpCode::CmpLt => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a < b { 1 } else { 0 });
                }
            }

            OpCode::CmpLe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a <= b { 1 } else { 0 });
                }
            }

            OpCode::CmpGt => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a > b { 1 } else { 0 });
                }
            }

            OpCode::CmpGe => {
                let a = self.get_value(&instr.operands[0]);
                let b = self.get_value(&instr.operands[1]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), if a >= b { 1 } else { 0 });
                }
            }

            OpCode::StoreVar => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
            }

            OpCode::Copy => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
            }

            OpCode::LoadVar => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
            }

            // Control flow handled in run_function
            OpCode::Jump | OpCode::Branch | OpCode::Return => {}
            
            // Labels are markers, no execution
            OpCode::Label | OpCode::Nop => {}

            OpCode::Call => {
                // Function calls to be implemented later
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

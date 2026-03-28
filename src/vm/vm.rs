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
                    println!(
                        "{:?} executed {} times",
                        instr.opcode,
                        instr.profile.exec_count
                    );
                }
            }
        }
    }

    fn run_function(&mut self, func: &mut FunctionIR) {
        for block in &mut func.blocks {
            for instr in &mut block.instructions {
                self.execute(instr);
            }
        }
    }

    fn execute(&mut self, instr: &mut Instruction) {
        instr.profile.exec_count += 1;

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

            OpCode::Return => {
                let val = self.get_value(&instr.operands[0]);
                println!("Program returned: {}", val);
            }

            // Control flow opcodes (Jump, Branch, Label) need block-level execution
            // For now, they are no-ops in single-block execution
            OpCode::Jump | OpCode::Branch | OpCode::Label | OpCode::Nop => {}

            OpCode::LoadVar => {
                let val = self.get_value(&instr.operands[0]);
                if let Some(name) = &instr.result {
                    self.variables.insert(name.clone(), val);
                }
            }

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

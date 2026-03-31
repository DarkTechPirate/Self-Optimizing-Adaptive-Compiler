use serde::Serialize;
use crate::lexer::Lexer;
use crate::lexer::token::Token;
use crate::parser::parser::Parser;
use crate::ir::lower::Lowerer;
use crate::ir::ir::ProgramIR;
use crate::vm::vm::NyxVM;
use crate::optimizer::Optimizer;

/// Result of compilation (JSON-serializable for LLM)
#[derive(Debug, Serialize)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip)]
    pub ir: Option<ProgramIR>,
    pub instruction_count: usize,
    pub error: Option<String>,
}

/// Result of execution (JSON-serializable for LLM)
#[derive(Debug, Serialize)]
pub struct ExecuteResult {
    pub success: bool,
    pub return_value: Option<i64>,
    pub total_instructions: u64,
    pub total_time_us: u64,  // microseconds for readability
    pub hot_instruction_count: usize,
    pub error: Option<String>,
}

/// Result of optimization (JSON-serializable for LLM)
#[derive(Debug, Serialize)]
pub struct OptimizeResult {
    pub success: bool,
    pub optimizations_applied: Vec<String>,
    pub instructions_before: usize,
    pub instructions_after: usize,
    pub instructions_removed: usize,
    pub error: Option<String>,
}

/// Profile data for a single instruction (JSON-serializable for LLM)
#[derive(Debug, Clone, Serialize)]
pub struct InstructionProfile {
    pub opcode: String,
    pub exec_count: u64,
    pub avg_time_ns: u64,
    pub is_hot: bool,
}

/// Result of profiling (JSON-serializable for LLM)
#[derive(Debug, Serialize)]
pub struct ProfileResult {
    pub success: bool,
    pub instructions: Vec<InstructionProfile>,
    pub total_instructions: u64,
    pub total_time_us: u64,  // microseconds for readability
    pub hot_count: usize,
    pub error: Option<String>,
}

/// Analysis suggestions for LLM
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub success: bool,
    pub suggestions: Vec<String>,
    pub hot_instructions: Vec<String>,
    pub optimization_opportunities: Vec<String>,
}

/// The Nyx Compiler API
pub struct NyxCompiler {
    ir: Option<ProgramIR>,
    vm: NyxVM,
}

impl NyxCompiler {
    pub fn new() -> Self {
        Self {
            ir: None,
            vm: NyxVM::new(),
        }
    }

    /// Compile source code to IR
    pub fn compile(&mut self, source: &str) -> CompileResult {
        // Lexer
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();

        loop {
            let tok = lexer.next_token();
            tokens.push(tok.clone());
            if tok == Token::EOF { break; }
        }

        // Parser
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();

        // Lower to IR
        let ir = Lowerer::lower_program(program);
        let count = Self::count_instructions(&ir);
        self.ir = Some(ir.clone());

        CompileResult {
            success: true,
            ir: Some(ir),
            instruction_count: count,
            error: None,
        }
    }

    /// Optimize the compiled IR
    pub fn optimize(&mut self) -> OptimizeResult {
        match &mut self.ir {
            Some(ir) => {
                let before = Self::count_instructions(ir);
                
                let mut optimizations = Vec::new();
                
                // Run optimization passes
                Optimizer::optimize(ir);
                
                optimizations.push("constant_folding".to_string());
                optimizations.push("dead_code_elimination".to_string());
                optimizations.push("loop_invariant_code_motion".to_string());
                optimizations.push("strength_reduction".to_string());

                let after = Self::count_instructions(ir);

                OptimizeResult {
                    success: true,
                    optimizations_applied: optimizations,
                    instructions_before: before,
                    instructions_after: after,
                    instructions_removed: before.saturating_sub(after),
                    error: None,
                }
            }
            None => OptimizeResult {
                success: false,
                optimizations_applied: vec![],
                instructions_before: 0,
                instructions_after: 0,
                instructions_removed: 0,
                error: Some("No IR to optimize. Call compile() first.".to_string()),
            }
        }
    }

    /// Execute the compiled (and optionally optimized) IR
    pub fn execute(&mut self) -> ExecuteResult {
        match &mut self.ir {
            Some(ir) => {
                self.vm.run_program(ir);

                // Gather stats
                let mut total_instructions = 0u64;
                let mut total_time_ns = 0u64;
                let mut hot_count = 0;
                let mut return_value = None;

                for func in &ir.functions {
                    for block in &func.blocks {
                        for instr in &block.instructions {
                            total_instructions += instr.profile.exec_count;
                            total_time_ns += instr.profile.total_time_ns;
                            if instr.profile.is_hot {
                                hot_count += 1;
                            }
                            if instr.profile.last_value.is_some() {
                                return_value = instr.profile.last_value;
                            }
                        }
                    }
                }

                ExecuteResult {
                    success: true,
                    return_value,
                    total_instructions,
                    total_time_us: total_time_ns / 1000,
                    hot_instruction_count: hot_count,
                    error: None,
                }
            }
            None => ExecuteResult {
                success: false,
                return_value: None,
                total_instructions: 0,
                total_time_us: 0,
                hot_instruction_count: 0,
                error: Some("No IR to execute. Call compile() first.".to_string()),
            }
        }
    }

    /// Get profiling data from last execution
    pub fn profile(&self) -> ProfileResult {
        match &self.ir {
            Some(ir) => {
                let mut instructions = Vec::new();
                let mut total_instructions = 0u64;
                let mut total_time_ns = 0u64;
                let mut hot_count = 0;

                for func in &ir.functions {
                    for block in &func.blocks {
                        for instr in &block.instructions {
                            if instr.profile.exec_count > 0 {
                                instructions.push(InstructionProfile {
                                    opcode: format!("{:?}", instr.opcode),
                                    exec_count: instr.profile.exec_count,
                                    avg_time_ns: instr.profile.avg_time_ns(),
                                    is_hot: instr.profile.is_hot,
                                });
                                total_instructions += instr.profile.exec_count;
                                total_time_ns += instr.profile.total_time_ns;
                                if instr.profile.is_hot {
                                    hot_count += 1;
                                }
                            }
                        }
                    }
                }

                ProfileResult {
                    success: true,
                    instructions,
                    total_instructions,
                    total_time_us: total_time_ns / 1000,
                    hot_count,
                    error: None,
                }
            }
            None => ProfileResult {
                success: false,
                instructions: vec![],
                total_instructions: 0,
                total_time_us: 0,
                hot_count: 0,
                error: Some("No IR to profile. Call compile() and execute() first.".to_string()),
            }
        }
    }

    /// Analyze IR for optimization opportunities (returns JSON-friendly result)
    pub fn analyze(&self) -> AnalysisResult {
        match &self.ir {
            Some(ir) => {
                let mut suggestions = Vec::new();
                let mut hot_instructions = Vec::new();
                let mut optimization_opportunities = Vec::new();
                
                for func in &ir.functions {
                    for block in &func.blocks {
                        for instr in &block.instructions {
                            if instr.profile.is_hot {
                                hot_instructions.push(format!("{:?}", instr.opcode));
                                suggestions.push(format!(
                                    "Hot {:?} ({}x) - consider optimization",
                                    instr.opcode, instr.profile.exec_count
                                ));
                            }
                        }
                    }
                }
                
                // Add general optimization opportunities
                if !hot_instructions.is_empty() {
                    optimization_opportunities.push("loop_unrolling".to_string());
                    optimization_opportunities.push("instruction_scheduling".to_string());
                }
                
                AnalysisResult {
                    success: true,
                    suggestions,
                    hot_instructions,
                    optimization_opportunities,
                }
            }
            None => AnalysisResult {
                success: false,
                suggestions: vec!["No IR to analyze".to_string()],
                hot_instructions: vec![],
                optimization_opportunities: vec![],
            },
        }
    }

    /// Get current IR (for inspection)
    pub fn get_ir(&self) -> Option<&ProgramIR> {
        self.ir.as_ref()
    }

    fn count_instructions(ir: &ProgramIR) -> usize {
        ir.functions.iter()
            .flat_map(|f| f.blocks.iter())
            .flat_map(|b| b.instructions.iter())
            .count()
    }
}

impl Default for NyxCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: compile and optimize code in one call
pub fn optimize(source: &str) -> OptimizeResult {
    let mut compiler = NyxCompiler::new();
    compiler.compile(source);
    compiler.optimize()
}

/// Convenience function: compile, optimize, and execute code
pub fn execute(source: &str) -> ExecuteResult {
    let mut compiler = NyxCompiler::new();
    compiler.compile(source);
    compiler.optimize();
    compiler.execute()
}

/// Convenience function: compile, execute, and profile code
pub fn profile(source: &str) -> ProfileResult {
    let mut compiler = NyxCompiler::new();
    compiler.compile(source);
    compiler.execute();
    compiler.profile()
}

// ============ JSON OUTPUT FOR LLM ============

/// Convert ExecuteResult to JSON string
pub fn execute_json(source: &str) -> String {
    let result = execute(source);
    serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Convert OptimizeResult to JSON string
pub fn optimize_json(source: &str) -> String {
    let result = optimize(source);
    serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Convert ProfileResult to JSON string  
pub fn profile_json(source: &str) -> String {
    let result = profile(source);
    serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Full pipeline: compile, optimize, execute, and return JSON
pub fn run_json(source: &str) -> String {
    let mut compiler = NyxCompiler::new();
    
    let compile = compiler.compile(source);
    let optimize = compiler.optimize();
    let execute = compiler.execute();
    let profile = compiler.profile();
    let analysis = compiler.analyze();
    
    let result = serde_json::json!({
        "compile": {
            "success": compile.success,
            "instruction_count": compile.instruction_count,
            "error": compile.error,
        },
        "optimize": optimize,
        "execute": execute,
        "profile": profile,
        "analysis": analysis,
    });
    
    serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

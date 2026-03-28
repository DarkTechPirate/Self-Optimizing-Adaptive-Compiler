mod lexer;
mod parser;
mod ast;
mod ir;
mod vm;
mod optimizer;
pub mod api;

use api::NyxCompiler;

fn main() {
    let source = r#"
        fn sum() {
            let total = 0
            for i in 0..5 {
                total = total + i
            }
            return total
        }
    "#;

    println!("=== Nyx Compiler API Demo ===\n");

    // Create compiler instance
    let mut compiler = NyxCompiler::new();

    // Step 1: Compile
    println!("1. Compiling source code...");
    let compile_result = compiler.compile(source);
    println!("   Compile success: {}", compile_result.success);

    // Step 2: Execute (before optimization)
    println!("\n2. Executing (unoptimized)...");
    let exec_result = compiler.execute();
    println!("   Return value: {:?}", exec_result.return_value);
    println!("   Instructions executed: {}", exec_result.total_instructions);

    // Step 3: Optimize
    println!("\n3. Optimizing...");
    let opt_result = compiler.optimize();
    println!("   Optimizations: {:?}", opt_result.optimizations_applied);
    println!("   Instructions: {} -> {}", opt_result.instructions_before, opt_result.instructions_after);

    // Step 4: Execute again (optimized)
    println!("\n4. Executing (optimized)...");
    let exec_result2 = compiler.execute();
    println!("   Return value: {:?}", exec_result2.return_value);
    println!("   Instructions executed: {}", exec_result2.total_instructions);
    println!("   Hot instructions: {}", exec_result2.hot_instruction_count);

    // Step 5: Get profile
    println!("\n5. Profiling...");
    let profile = compiler.profile();
    println!("   Total time: {}μs", profile.total_time_ns / 1000);
    println!("   Hot instructions: {}", profile.hot_count);

    // Using convenience functions
    println!("\n=== Using Convenience Functions ===");
    let result = api::execute(source);
    println!("api::execute() returned: {:?}", result.return_value);
}

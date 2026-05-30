use nyx::api::NyxCompiler;
use nyx::optimizer::OptimizationPlan;
use std::path::PathBuf;

fn read_sample(path: &str) -> String {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(base.join(path)).expect("failed to read sample")
}

fn assert_no_opt(path: &str) {
    let source = read_sample(path);
    let mut compiler = NyxCompiler::new();
    let compile = compiler.compile(&source);
    assert!(compile.success, "compile failed for {}", path);

    let baseline = compiler.execute();
    let optimize = compiler.optimize_with_plan(OptimizationPlan::baseline());
    assert!(optimize.success, "optimize failed for {}", path);
    let optimized = compiler.execute();

    assert_eq!(baseline.return_value, optimized.return_value, "return mismatch for {}", path);
    assert!(
        optimize.instructions_after <= optimize.instructions_before,
        "instructions grew for {}",
        path
    );
}

#[test]
fn no_opt_samples_preserve_results() {
    assert_no_opt("samples/validation/08_already_optimized_linear.nyx");
    assert_no_opt("samples/validation/09_no_optimization_possible.nyx");
}

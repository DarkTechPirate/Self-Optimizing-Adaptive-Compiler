use nyx::api::NyxCompiler;
use nyx::optimizer::OptimizationPlan;
use proptest::prelude::*;

fn expr_strategy() -> impl Strategy<Value = String> {
    let num = -20i64..20i64;
    let op = prop_oneof![Just("+"), Just("-"), Just("*"), Just("/")];

    prop::collection::vec(num, 1..6).prop_flat_map(move |nums| {
        let len = nums.len();
        (
            Just(nums),
            prop::collection::vec(op.clone(), len.saturating_sub(1)),
        )
    }).prop_map(|(nums, ops)| {
        let mut expr = nums[0].to_string();
        for (idx, op) in ops.iter().enumerate() {
            expr = format!("({} {} {})", expr, op, nums[idx + 1]);
        }
        expr
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn optimize_preserves_result(expr in expr_strategy()) {
        let source = format!("fn main() {{\nlet x = {}\nreturn x\n}}", expr);
        let mut compiler = NyxCompiler::new();
        let compile = compiler.compile(&source);
        prop_assert!(compile.success);

        let baseline = compiler.execute();
        let plan = OptimizationPlan::aggressive();
        let optimize = compiler.optimize_with_plan(plan);
        prop_assert!(optimize.success);
        let optimized = compiler.execute();

        prop_assert_eq!(baseline.return_value, optimized.return_value);
    }
}

# Validation Guide

Run proof-of-value testing before launch.

## Command
```bash
./scripts/proof_of_value_benchmark.sh
```

Optional output directory:
```bash
./scripts/proof_of_value_benchmark.sh .nyx/custom_validation
```

## What it runs
1. Controlled tests (10 programs, no LLM)
2. Product-lane tests (10 programs, auto mode with LLM path enabled)
3. Consistency test (same program, 3 sequential runs with retained history)
4. Failure checks (already-optimized + no-optimization cases)

## Output files
- `results.jsonl`: one JSON record per run
- `summary.json`: aggregate metrics and launch verdict

Default location:
- `.nyx/validation/results.jsonl`
- `.nyx/validation/summary.json`

## Launch gates
- `controlled_average_speedup >= 1.5`
- `correctness_pass_rate == 1.0`
- `consistency_ok == true`
- `failure_checks.no_optimization_case_ok == true`
- `failure_checks.already_optimized_case_ok == true`

Consistency gate details:
- run 3 speedup should remain within 10 percent of median consistency speedup
- minimum consistency speedup should stay >= 1.0
- `consistency_variation_ratio <= 0.40`
- `consistency_history_reused == true`

No-degradation gate details:
- correctness must remain true
- optimized runtime may be at most 20 percent slower or 150 microseconds slower (whichever is larger), to account for benchmark jitter

If any gate fails, `launch_gate_pass` is false and `fail_reasons` explains why.

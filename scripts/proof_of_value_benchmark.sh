#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-.nyx/validation}"
RESULTS_FILE="$OUT_DIR/results.jsonl"
SUMMARY_FILE="$OUT_DIR/summary.json"

PROGRAMS=(
  "samples/validation/01_redundant_scalar_repeat.nyx"
  "samples/validation/02_redundant_loop_hotpath.nyx"
  "samples/validation/03_dead_code_unreachable.nyx"
  "samples/validation/04_loop_invariant_constant.nyx"
  "samples/validation/05_strength_reduction_mul.nyx"
  "samples/validation/06_mixed_loop_if_redundancy.nyx"
  "samples/validation/07_mixed_with_dead_code.nyx"
  "samples/validation/08_already_optimized_linear.nyx"
  "samples/validation/09_no_optimization_possible.nyx"
  "samples/validation/10_realistic_python_snippet.py"
)

run_nyx() {
  local log_file="$1"
  shift

  if [[ "${NYX_USE_RELEASE:-0}" == "1" && -x "./target/release/nyx" ]]; then
    ./target/release/nyx --log-file "$log_file" "$@"
  else
    cargo run -q -- --log-file "$log_file" "$@"
  fi
}

record_result() {
  local program="$1"
  local lane="$2"
  local run_index="$3"
  local log_file="$4"
  local disable_llm="$5"

  local args=(optimize "$program" --mode auto)
  if [[ "$disable_llm" == "true" ]]; then
    args+=(--no-llm)
  fi

  local raw
  raw="$(run_nyx "$log_file" "${args[@]}" 2>/dev/null)"

  local record
  record="$(RAW_JSON="$raw" python3 - "$program" "$lane" "$run_index" <<'PY'
import json
import os
import sys

program = sys.argv[1]
lane = sys.argv[2]
run_index = int(sys.argv[3])
payload = json.loads(os.environ["RAW_JSON"])

record = {
    "program": program,
    "lane": lane,
    "run_index": run_index,
    "baseline_time_us": payload.get("execution_time_before_us"),
    "optimized_time_us": payload.get("execution_time_after_us"),
    "speedup": payload.get("speedup_ratio"),
    "strategies_used": payload.get("selected_strategies", []),
    "llm_status": payload.get("llm_status"),
    "history_reused": payload.get("reused_history"),
    "baseline_return_value": payload.get("baseline_return_value"),
    "optimized_return_value": payload.get("optimized_return_value"),
    "correctness_verified": payload.get("correctness_verified"),
}

print(json.dumps(record, separators=(",", ":")))
PY
)"

  printf '%s\n' "$record" >> "$RESULTS_FILE"
  printf '%s\n' "$record"
}

prepare_run_dir() {
  local lane="$1"
  local name="$2"
  local run_dir="$OUT_DIR/runs/$lane/$name"
  rm -rf "$run_dir"
  mkdir -p "$run_dir"
  printf '%s' "$run_dir"
}

mkdir -p "$OUT_DIR"
: > "$RESULTS_FILE"

echo "== Controlled testing (no LLM) =="
for program in "${PROGRAMS[@]}"; do
  name="$(basename "$program")"
  run_dir="$(prepare_run_dir "controlled" "$name")"
  record_result "$program" "controlled" 1 "$run_dir/metrics.jsonl" true >/dev/null
  echo "controlled: $program"
done

echo
echo "== Product-lane testing (auto mode, LLM path enabled) =="
for program in "${PROGRAMS[@]}"; do
  name="$(basename "$program")"
  run_dir="$(prepare_run_dir "product" "$name")"
  record_result "$program" "product" 1 "$run_dir/metrics.jsonl" false >/dev/null
  echo "product: $program"
done

echo
echo "== Consistency testing (3 runs, retained history) =="
CONSISTENCY_PROGRAM="samples/validation/02_redundant_loop_hotpath.nyx"
consistency_dir="$OUT_DIR/runs/consistency/redundant_loop_hotpath"
rm -rf "$consistency_dir"
mkdir -p "$consistency_dir"

for run in 1 2 3; do
  record_result "$CONSISTENCY_PROGRAM" "consistency" "$run" "$consistency_dir/metrics.jsonl" true >/dev/null
  echo "consistency run $run complete"
done

python3 - "$RESULTS_FILE" "$SUMMARY_FILE" <<'PY'
import json
import statistics
import sys

results_path = sys.argv[1]
summary_path = sys.argv[2]

rows = []
with open(results_path, "r", encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if line:
            rows.append(json.loads(line))

controlled = [r for r in rows if r["lane"] == "controlled"]
product = [r for r in rows if r["lane"] == "product"]
consistency = sorted(
    [r for r in rows if r["lane"] == "consistency"],
    key=lambda r: r["run_index"],
)

speedups_controlled = [r["speedup"] for r in controlled if isinstance(r.get("speedup"), (int, float))]
speedups_product = [r["speedup"] for r in product if isinstance(r.get("speedup"), (int, float))]
all_correct = [bool(r.get("correctness_verified")) for r in rows]
correctness_pass_rate = sum(all_correct) / len(all_correct) if all_correct else 0.0

avg_controlled = statistics.fmean(speedups_controlled) if speedups_controlled else 0.0
avg_product = statistics.fmean(speedups_product) if speedups_product else 0.0

consistency_speedups = [r.get("speedup") for r in consistency]
consistency_ok = False
consistency_variation_ratio = None
consistency_median_speedup = None
consistency_history_reused = any(bool(r.get("history_reused")) for r in consistency[1:])
if len(consistency_speedups) == 3 and all(isinstance(v, (int, float)) for v in consistency_speedups):
    run1, run2, run3 = consistency_speedups
    min_speedup = min(consistency_speedups)
    max_speedup = max(consistency_speedups)
    consistency_median_speedup = statistics.median(consistency_speedups)
    consistency_variation_ratio = ((max_speedup - min_speedup) / max_speedup) if max_speedup > 0 else 1.0
    # Stability definition: run3 should stay near the median historical behavior,
    # variance should be bounded, and history must be reused after warm-up.
    consistency_ok = (
        run3 >= (consistency_median_speedup * 0.90)
        and min_speedup >= 1.0
        and consistency_variation_ratio <= 0.40
        and consistency_history_reused
    )

lookup = {r["program"]: r for r in controlled}
no_opt_program = "samples/validation/09_no_optimization_possible.nyx"
already_opt_program = "samples/validation/08_already_optimized_linear.nyx"

no_opt_row = lookup.get(no_opt_program, {})
already_opt_row = lookup.get(already_opt_program, {})

def non_degradation_ok(row):
    if not bool(row.get("correctness_verified")):
        return False, None

    baseline = row.get("baseline_time_us")
    optimized = row.get("optimized_time_us")
    if not isinstance(baseline, (int, float)) or not isinstance(optimized, (int, float)):
        return False, None

    # Allow small benchmark jitter: <=20% slower or <=150us slower, whichever is larger.
    allowed_slowdown_us = max(150.0, baseline * 0.20)
    slowdown_us = optimized - baseline
    return slowdown_us <= allowed_slowdown_us, slowdown_us

no_opt_ok, no_opt_slowdown_us = non_degradation_ok(no_opt_row)
already_opt_ok, already_opt_slowdown_us = non_degradation_ok(already_opt_row)

fail_reasons = []
if avg_controlled < 1.5:
    fail_reasons.append("average_speedup_below_1_5x")
if correctness_pass_rate < 1.0:
    fail_reasons.append("correctness_not_100_percent")
if not consistency_ok:
    fail_reasons.append("consistency_not_stable")
if not no_opt_ok:
    fail_reasons.append("no_optimization_case_degraded_or_incorrect")
if not already_opt_ok:
    fail_reasons.append("already_optimized_case_degraded_or_incorrect")

summary = {
    "program_count": len(controlled),
    "controlled_average_speedup": avg_controlled,
    "product_average_speedup": avg_product,
    "correctness_pass_rate": correctness_pass_rate,
    "consistency_speedups": consistency_speedups,
    "consistency_median_speedup": consistency_median_speedup,
    "consistency_ok": consistency_ok,
    "consistency_variation_ratio": consistency_variation_ratio,
    "consistency_history_reused": consistency_history_reused,
    "failure_checks": {
        "no_optimization_case_ok": no_opt_ok,
        "no_optimization_case_slowdown_us": no_opt_slowdown_us,
        "already_optimized_case_ok": already_opt_ok,
        "already_optimized_case_slowdown_us": already_opt_slowdown_us,
    },
    "launch_gate_pass": len(fail_reasons) == 0,
    "fail_reasons": fail_reasons,
}

with open(summary_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, indent=2)

print("=== Proof-of-Value Summary ===")
print(f"programs: {summary['program_count']}")
print(f"controlled_average_speedup: {summary['controlled_average_speedup']:.3f}x")
print(f"product_average_speedup: {summary['product_average_speedup']:.3f}x")
print(f"correctness_pass_rate: {summary['correctness_pass_rate']:.3f}")
print(f"consistency_speedups: {summary['consistency_speedups']}")
print(f"consistency_ok: {summary['consistency_ok']}")
print(f"launch_gate_pass: {summary['launch_gate_pass']}")
if summary["fail_reasons"]:
    print("fail_reasons:")
    for reason in summary["fail_reasons"]:
        print(f"- {reason}")
PY

echo
echo "results: $RESULTS_FILE"
echo "summary: $SUMMARY_FILE"

#!/usr/bin/env bash
set -euo pipefail

PROGRAM_FILE="${1:-samples/redundant_hotpath.nyx}"

run_nyx() {
  cargo run -q -- "$@" 2>/dev/null
}

echo "1) Baseline run without optimization"
BASELINE_JSON="$(run_nyx analyze "$PROGRAM_FILE" --no-llm)"
BASELINE_TIME_US="$(echo "$BASELINE_JSON" | sed -n 's/.*"execution_time_us": \([0-9][0-9]*\).*/\1/p' | head -n 1)"
echo "baseline_time_us=${BASELINE_TIME_US:-unknown}"

echo
echo "2) Run with Nyx auto optimization"
OPTIMIZED_JSON="$(run_nyx optimize "$PROGRAM_FILE" --mode auto --no-llm)"

echo
echo "3) Highlighted result"
SPEEDUP="$(echo "$OPTIMIZED_JSON" | sed -n 's/.*"speedup_ratio": \([0-9.][0-9.]*\).*/\1/p' | head -n 1)"
REUSED="$(echo "$OPTIMIZED_JSON" | sed -n 's/.*"reused_history": \(true\|false\).*/\1/p' | head -n 1)"
STRATEGIES="$(
  echo "$OPTIMIZED_JSON" \
    | sed -n '/"selected_strategies": \[/,/\],/p' \
    | tr '\n' ' ' \
    | sed 's/.*\[//; s/\],.*//; s/  */ /g; s/^ *//; s/ *$//'
)"

echo "{"
echo "  \"speedup\": \"${SPEEDUP:-0}x\"," 
echo "  \"selected_strategies\": [${STRATEGIES}],"
echo "  \"history_reused\": ${REUSED:-false}"
echo "}"

echo
echo "4) Full optimize payload"
echo "$OPTIMIZED_JSON"

echo
echo "5) Dashboard"
echo "Run: nyx serve --host 127.0.0.1 --port 8090"
echo "Open: http://127.0.0.1:8090/dashboard"

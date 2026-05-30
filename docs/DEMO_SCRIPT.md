# Nyx Demo Script (Phase 4)

Use this exact flow for live demos and videos.

## Input
Inefficient program with redundancy + loops.

Recommended sample inputs:
- `samples/redundant_hotpath.nyx`
- `samples/redundant_hotpath.py` (Python input path)

## Step 1: Run without optimization
```bash
nyx analyze samples/redundant_hotpath.nyx --no-llm
```

Callout in demo:
- Show `execution_time_us` (example: `1200us`).

## Step 2: Run with Nyx auto optimization
```bash
nyx optimize samples/redundant_hotpath.nyx --mode auto
```

Callout in demo:
- Show `speedup_ratio`
- Show `selected_strategies`
- Show `reused_history`
- Show `correctness_verified`

## Step 3: Show the product JSON moment
Expected shape:
```json
{
  "speedup": "4.2x",
  "selected_strategies": ["cse", "constant_folding"],
  "history_reused": true
}
```

Nyx currently emits:
- `speedup_ratio`
- `selected_strategies`
- `reused_history`
- `correctness_verified`
- `baseline_return_value`
- `optimized_return_value`

Use this display conversion in your narration:
- `speedup = speedup_ratio + "x"`
- `history_reused = reused_history`
- `safe = correctness_verified`

## Step 3.5: Show proof-of-value validation
```bash
./scripts/proof_of_value_benchmark.sh
```

Callout in demo:
- Show `controlled_average_speedup` from `.nyx/validation/summary.json`
- Show `correctness_pass_rate`
- Show `launch_gate_pass`

## Step 4: Show dashboard proof
Start API server:
```bash
nyx serve --host 127.0.0.1 --port 8090
```

Open:
- `http://127.0.0.1:8090/dashboard`

Highlight these sections:
- Strategy success rate
- Learning reuse
- LLM reasoning
- Nyx Saved You Today

## 30-second narration
"Nyx is a self-learning runtime. First run: baseline performance. Second run with auto mode: Nyx selects strategies, reuses what worked before, and speeds up execution. On the dashboard you can see strategy win rates, reasoning traces, and total time saved."

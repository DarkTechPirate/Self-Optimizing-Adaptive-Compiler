# Nyx Runtime

Nyx is a self-learning runtime that automatically makes your code faster over time.

Short version: your code gets faster every time it runs.

## What it is
Nyx profiles real execution, selects optimization strategies automatically, and reuses what previously worked. It targets a Nyx DSL and simple Python inputs.

## What it does
- Profiles execution in real time
- Selects optimization strategies automatically (`--mode auto`)
- Learns from previous runs and reuses successful strategies
- Tracks saved time and exposes it in API and dashboard
- Accepts Nyx DSL and simple Python input

## Tech stack
- Rust core (`nyx` crate) with CLI via `clap` and JSON output via `serde` / `serde_json`
- API server built on `axum` + `tokio` with REST endpoints and a dashboard
- `reqwest` for outbound HTTP (optional LLM path) and `sysinfo` for system metrics
- Optional local LLM integration via Ollama (not required for core features)

## How it works (high-level flow)
- Run `nyx analyze` to profile execution
- Run `nyx optimize --mode auto` to select and apply strategies
- Results include speedup, selected strategies, and correctness verification
- Exposes `POST /execute`, `POST /analyze`, `POST /optimize`, plus metrics and dashboard endpoints
- Proof-of-value validation is scripted and gate-checked via `./scripts/proof_of_value_benchmark.sh`

## Quick start
```bash
cargo build --release
./target/release/nyx analyze samples/redundant_hotpath.nyx --no-llm
./target/release/nyx optimize samples/redundant_hotpath.nyx --mode auto --no-llm
./target/release/nyx serve --host 127.0.0.1 --port 8090
```

Dashboard:
- `http://127.0.0.1:8090/dashboard`

## Core commands
```bash
nyx run <file> [--mode auto|speed|memory|balanced]
nyx analyze <file> [--no-llm]
nyx optimize <file> [--mode auto|speed|memory|balanced] [--no-llm]
nyx serve --host 127.0.0.1 --port 8090
```

## Proof-of-value validation
Run the benchmark harness:

```bash
./scripts/proof_of_value_benchmark.sh
```

Generated artifacts:
- `.nyx/validation/results.jsonl`
- `.nyx/validation/summary.json`

Launch gate defaults:
- average speedup >= 1.5x
- correctness pass rate = 100%
- consistency runs must stabilize within bounded variance and show history reuse
- no-op and already-optimized cases must not materially degrade (jitter-aware threshold)

## Phase 4 assets
- `docs/DEMO_SCRIPT.md`
- `docs/POSITIONING.md`
- `docs/LAUNCH.md`
- `docs/FIRST_10_USERS.md`
- `docs/NEXT_3_DAYS.md`
- `docs/API.md`
- `docs/INSTALL.md`
- `docs/VALIDATION.md`
- `scripts/demo_phase4.sh`
- `scripts/proof_of_value_benchmark.sh`

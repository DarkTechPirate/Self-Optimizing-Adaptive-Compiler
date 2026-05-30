# Install and Run

## Requirements
- Rust toolchain
- Optional: Ollama for LLM suggestions

## Build
```bash
cargo build --release
```

Binary:
- `target/release/nyx`

## Optional local install
```bash
cargo install --path .
```

## Quick start
```bash
nyx analyze samples/redundant_hotpath.nyx --no-llm
nyx optimize samples/redundant_hotpath.nyx --mode auto --no-llm
nyx serve --host 127.0.0.1 --port 8090
```

Dashboard:
- `http://127.0.0.1:8090/dashboard`

# Self-Optimizing Edge AI System - Implementation Plan

## 🚀 QUICK START FOR NEW SESSION
**Last Updated:** 2026-03-28 10:31 UTC

### Current Status: Phase 2 COMPLETE ✅ | Next: Phase 3 (LLM Integration)
**Progress: 8/15 tasks done (53%)**

### Next Task: P3-T1 - Set up TinyLLaMA locally
```bash
# To continue, run:
cd /Users/jk/Projects/Complier/Self-Optimizing-Adaptive-Compiler.worktrees/copilot-worktree-2026-03-28T07-38-15
cargo run  # Test current state
```

---

## Project Overview
Building a **Self-Optimizing AI Runtime System** that combines:
- **Nyx Compiler** → optimizes execution dynamically
- **LLM (TinyLLaMA)** → reasoning/decisions for optimization strategies  
- **Edge Hardware** → runs locally on QCS/ESP32

---

## 📁 Project Structure
```
src/
├── api/mod.rs       # ✅ NyxCompiler API with JSON output
├── ast/node.rs      # ✅ AST nodes (Expr, Stmt, While, For, If)
├── ir/
│   ├── ir.rs        # ✅ IR types (20+ opcodes, ProfileData)
│   └── lower.rs     # ✅ AST → IR lowering with control flow
├── lexer/
│   ├── mod.rs       # ✅ Tokenizer (operators, keywords)
│   └── token.rs     # ✅ Token types
├── optimizer/mod.rs # ✅ Optimization passes (DCE, LICM, strength reduction)
├── parser/parser.rs # ✅ Parser (loops, if/else, expressions)
├── vm/vm.rs         # ✅ VM with control flow & profiling
└── main.rs          # Demo of API
```

---

## ✅ COMPLETED PHASES

### Phase 1: Stabilize & Extend Compiler ✅ DONE
| Task | Description | Status |
|------|-------------|--------|
| P1-T1 | Dead Code Elimination | ✅ done |
| P1-T2 | Loop Optimization (While/For, LICM) | ✅ done |
| P1-T3 | Enhanced IR (20+ opcodes) | ✅ done |
| P1-T4 | Instruction profiling (timing, hot detection) | ✅ done |

**What was implemented:**
- Opcodes: Add, Sub, Mul, Div, Mod, Neg, CmpEq/Ne/Lt/Le/Gt/Ge, Jump, Branch, Label, LoadConst, LoadVar, StoreVar, Copy, Call, Return, Nop
- While loops: `while condition { body }`
- For loops: `for i in start..end { body }`
- If/Else: `if condition { } else { }`
- DCE removes unused vars and unreachable code
- LICM hoists loop-invariant code
- Strength reduction: x*2 → x+x

### Phase 2: Compiler API ✅ DONE
| Task | Description | Status |
|------|-------------|--------|
| P2-T1 | optimize(code) API | ✅ done |
| P2-T2 | execute(code) API | ✅ done |
| P2-T3 | profile(code) API | ✅ done |
| P2-T4 | JSON output for LLM | ✅ done |

**API Usage:**
```rust
use nyx::api::{NyxCompiler, execute_json, run_json};

// Method 1: Full control
let mut compiler = NyxCompiler::new();
compiler.compile(source);
compiler.optimize();
let result = compiler.execute();

// Method 2: JSON for LLM
let json = run_json(source);  // Returns full pipeline JSON
```

**JSON Output Structure:**
```json
{
  "compile": { "success": true, "instruction_count": 17 },
  "optimize": { "instructions_before": 17, "instructions_after": 17, "instructions_removed": 0 },
  "execute": { "return_value": 10, "total_time_us": 95, "hot_instruction_count": 9 },
  "profile": { "instructions": [...], "hot_count": 9 },
  "analysis": { "hot_instructions": ["Label", "CmpLt", ...], "optimization_opportunities": [...] }
}
```

---

## ⏳ PENDING PHASES

### Phase 3: Integrate LLM (TinyLLaMA) ⬜ NEXT
| Task | Description | Status |
|------|-------------|--------|
| P3-T1 | Set up TinyLLaMA locally (Mac) | ⬜ pending |
| P3-T2 | Create LLM → Compiler interface | ⬜ pending |
| P3-T3 | Implement strategy suggestion | ⬜ pending |
| P3-T4 | Test end-to-end flow | ⬜ pending |

**How to implement P3-T1:**
1. Install Ollama on Mac: `brew install ollama`
2. Start Ollama server: `ollama serve`
3. Pull TinyLLaMA model: `ollama pull tinyllama`
4. Test: `ollama run tinyllama "Hello"`
5. Create Rust HTTP client to call Ollama API

**P3-T2 Interface Design:**
```rust
// LLM receives JSON from compiler
let profile_json = compiler.profile_json();
let llm_response = llm.query(&format!("Optimize this: {}", profile_json));

// LLM returns optimization suggestions
// { "suggestions": ["unroll_loop", "inline_function"] }

// Compiler applies suggestions
compiler.apply_optimizations(llm_response.suggestions);
```

**P3-T3 Strategy Suggestion System:**
```rust
pub struct LLMSuggestion {
    pub strategy: String,      // e.g., "unroll_loop"
    pub target: String,        // e.g., "loop_at_line_5"
    pub confidence: f32,       // 0.0 - 1.0
}

// LLM analyzes profile data and suggests optimizations
fn suggest_optimizations(profile: &ProfileResult) -> Vec<LLMSuggestion>;
```

**P3-T4 End-to-End Flow:**
```
User: "Make this faster"
    ↓
Compiler: profile(code) → JSON
    ↓
LLM: analyze JSON → suggestions
    ↓
Compiler: apply_optimizations(suggestions)
    ↓
Output: optimized code + metrics
```

### Phase 4: Deploy on Edge Hardware ⬜
| Task | Description | Status |
|------|-------------|--------|
| P4-T1 | Prototype on Raspberry Pi 5 | ⬜ pending |
| P4-T2 | Port to Qualcomm QCS6490/QCS8550 | ⬜ pending |
| P4-T3 | Optimize for edge constraints | ⬜ pending |

**P4-T1 Steps:**
1. Cross-compile Rust for ARM64
2. Set up Raspberry Pi 5 with Rust runtime
3. Deploy and test Nyx compiler
4. Benchmark performance

**P4-T2 Steps:**
1. Get Qualcomm QCS SDK
2. Adapt build system for QCS target
3. Integrate with Hexagon DSP if available

**P4-T3 Optimizations:**
- Reduce memory footprint
- Use fixed-point arithmetic where possible
- Profile and optimize hot paths

---

## Architecture Diagram
```
[ Layer 1 ]  ESP32 (optional)
- Sensors / Data collection
        ↓
[ Layer 2 ]  QCS / Edge Processor
- Runs TinyLLaMA
- Runs Nyx compiler
- Handles inference + optimization
        ↓
[ Layer 3 ]  Execution Engine
- Compiler optimizes tasks dynamically
- Generates efficient execution plans
        ↓
[ Output ]
- Decision / control / response
```

---

## Key Files Reference

### src/api/mod.rs - Compiler API
```rust
pub struct NyxCompiler { ... }

impl NyxCompiler {
    pub fn new() -> Self;
    pub fn compile(&mut self, source: &str) -> CompileResult;
    pub fn optimize(&mut self) -> OptimizeResult;
    pub fn execute(&mut self) -> ExecuteResult;
    pub fn profile(&self) -> ProfileResult;
    pub fn analyze(&self) -> AnalysisResult;
}

// Convenience functions
pub fn execute_json(source: &str) -> String;
pub fn optimize_json(source: &str) -> String;
pub fn profile_json(source: &str) -> String;
pub fn run_json(source: &str) -> String;  // Full pipeline
```

### src/optimizer/mod.rs - Optimization Passes
```rust
pub fn constant_folding(block: &mut BasicBlock);
pub fn dead_code_elimination(block: &mut BasicBlock);
pub fn loop_invariant_code_motion(block: &mut BasicBlock);
pub fn strength_reduction(block: &mut BasicBlock);
```

### src/ir/ir.rs - IR Types
```rust
pub enum OpCode {
    LoadConst, LoadVar, StoreVar,
    Add, Sub, Mul, Div, Mod, Neg,
    CmpEq, CmpNe, CmpLt, CmpLe, CmpGt, CmpGe,
    Jump, Branch, Label,
    Copy, Call, Return, Nop,
}

pub struct ProfileData {
    pub exec_count: usize,
    pub total_time_ns: u128,
    pub last_value: Option<i64>,
    pub is_hot: bool,
}
```

---

## Git History
```
e6ac21c feat: add JSON output for LLM integration
632b74f feat: add NyxCompiler API for programmatic access
7b9ac5d feat: add detailed instruction profiling metrics
31ad949 feat: add loop support with While/For statements and loop optimizations
8ee05ef feat: add DCE optimization and enhance IR with more opcodes
```

---

## Progress Log
| Date | Time | Task | Status |
|------|------|------|--------|
| 2026-03-28 | 07:47 | Plan created | ✅ |
| 2026-03-28 | 08:00 | P1-T1 DCE | ✅ |
| 2026-03-28 | 08:25 | P1-T3 Enhanced IR | ✅ |
| 2026-03-28 | 08:35 | P1-T2 Loop optimization | ✅ |
| 2026-03-28 | 10:05 | P1-T4 Profiling | ✅ |
| 2026-03-28 | 10:10 | P2-T1/T2/T3 API | ✅ |
| 2026-03-28 | 10:18 | P2-T4 JSON output | ✅ |
| 2026-03-28 | 10:31 | Created TASKS.md | ✅ |

---

## Important Notes
⚠️ Do NOT:
- Run LLM on ESP32 (too weak)
- Jump to hardware before software is stable
- Build everything at once

✅ Focus order: Phase 1 ✅ → Phase 2 ✅ → Phase 3 ⬜ → Phase 4 ⬜

---

## Dependencies
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# For Phase 3, add:
# reqwest = { version = "0.11", features = ["json"] }
# tokio = { version = "1", features = ["full"] }
```

---

## Test Commands
```bash
# Build and run
cargo run

# Run tests
cargo test

# Check compilation
cargo check

# Build release
cargo build --release
```

---

## How to Continue Work

1. **Read this file** to understand current state
2. **Check next pending task** in the tables above
3. **Implement the task**
4. **Update this file** with new status
5. **Commit changes**

Current next step: **P3-T1 - Set up TinyLLaMA locally**

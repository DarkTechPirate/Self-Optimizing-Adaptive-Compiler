#[derive(Debug, Clone)]
pub enum IntentTag {
    Speed,
    Parallel,
    MemoryLow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Comparison (result: 1 = true, 0 = false)
    CmpEq,   // ==
    CmpNe,   // !=
    CmpLt,   // <
    CmpLe,   // <=
    CmpGt,   // >
    CmpGe,   // >=

    // Control flow
    Jump,          // unconditional jump to label
    Branch,        // conditional branch: if operand != 0, jump to label
    Label,         // marks a jump target

    // Data movement
    LoadConst,
    LoadVar,
    StoreVar,
    Copy,          // copy value from one var to another

    // Function
    Call,          // call function with args
    Return,

    // No-op (for optimization placeholders)
    Nop,
}

#[derive(Debug, Clone)]
pub struct ProfileData {
    pub exec_count: u64,
    pub total_time_ns: u64,      // cumulative execution time in nanoseconds
    pub last_value: Option<i64>, // last computed value (for analysis)
    pub is_hot: bool,            // marked hot if exec_count > threshold
}

impl ProfileData {
    pub fn new() -> Self {
        Self {
            exec_count: 0,
            total_time_ns: 0,
            last_value: None,
            is_hot: false,
        }
    }

    pub fn avg_time_ns(&self) -> u64 {
        if self.exec_count > 0 {
            self.total_time_ns / self.exec_count
        } else {
            0
        }
    }
}

impl Default for ProfileData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operands: Vec<String>,
    pub result: Option<String>,
    pub intents: Vec<IntentTag>,
    pub profile: ProfileData,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: Option<String>,  // optional label for jump targets
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    pub params: Vec<String>,
    pub blocks: Vec<BasicBlock>,
    pub intents: Vec<IntentTag>,
}

#[derive(Debug, Clone)]
pub struct ProgramIR {
    pub functions: Vec<FunctionIR>,
}

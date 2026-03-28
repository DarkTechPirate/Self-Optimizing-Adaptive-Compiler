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

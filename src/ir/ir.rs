#[derive(Debug, Clone)]
pub enum IntentTag {
    Speed,
    Parallel,
    MemoryLow,
}

#[derive(Debug, Clone)]
pub enum OpCode {
    Add,
    Return,
    LoadConst,
    LoadVar,
    StoreVar,
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

use lalrpop_util::lalrpop_mod;

lalrpop_mod!(assembly_grammar);

use super::{Nibble, OctDigit};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label {
    label: String,
}
impl Label {
    fn new(label: String) -> Result<Self, String> {
        Ok(Self { label })
    }
    pub fn to_string(&self) -> &String {
        &self.label
    }
}

#[derive(Debug, Clone)]
pub enum Condition {
    InputReady,    // I
    InputNotReady, // !I
    Equal,         // Z
    NotEqual,      // !Z
    Negative,      // N
    Positive,      // !N
    OverflowSet,   // V
    OverflowClear, // !V
    HigherSame,    // C
    Lower,         // !C
    Higher,        // C&!Z
    LowerSame,     // !C|Z
    GreaterEqual,  // N=V
    Less,          // N!=V
    Greater,       // N=V&!Z
    LessEqual,     // N!=V|Z
}

#[derive(Debug, Clone)]
pub enum Command {
    Pass,
    Value(u16),
    Jump(Label),
    Branch(Condition, Label),
    Push(Nibble),
    Pop(Nibble),
    Call(Label),
    Return,
    Add(Nibble),
    Rotate { shift: Nibble, register: Nibble },

    // ALM1
    Duplicate,
    Not,
    Read,
    ReadPop,
    Increment,
    IncrementWithCarry,
    Decrement,
    DecrementWithCarry,
    Negate,
    NegateWithCarry,
    NoopSetFlags,
    PopSetFlags,
    RightShift,
    RightShiftCarryIn,
    RightShiftOneIn,
    ArithmeticRightShift,

    // ALM2
    Swap(Nibble),
    Sub(Nibble),
    Write(Nibble),
    WritePop(Nibble),
    And(Nibble),
    Nand(Nibble),
    Or(Nibble),
    Nor(Nibble),
    Xor(Nibble),
    NXor(Nibble),
    RegToFlags(Nibble),
    Compare(Nibble),
    SwapAdd(Nibble),
    SwapSub(Nibble),
    AddWithCarry(Nibble),
    SubWithCarry(Nibble),

    RawRamCall,
    Input,
    Output(Vec<OctDigit>),
}

#[derive(Debug, Clone)]
pub enum Meta {
    AutoPage,
    RomPage(Nibble),
    RamPage,
    Label(Label),
    UseFlags,
    Comment,
}

#[derive(Debug, Clone)]
pub enum Line {
    Command(Command),
    Meta(Meta),
}

#[derive(Debug, Clone)]
pub struct Assembly {
    lines: Vec<Line>,
}

impl Assembly {
    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }
    fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }
}

pub fn load_assembly(source: &str) -> Assembly {
    assembly_grammar::AssemblyParser::new()
        .parse(source)
        .unwrap()
}

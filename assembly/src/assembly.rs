use lalrpop_util::{lalrpop_mod, lexer::Token};
lalrpop_mod!(assembly_grammar);
use crate::datatypes::{Nibble, OctDigit};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WithPos<T> {
    pub start: usize,
    pub end: usize,
    pub t: T,
}

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
    Value(WithPos<u16>),
    Jump(WithPos<Label>),
    Branch(WithPos<Condition>, WithPos<Label>),
    Push(WithPos<Nibble>),
    Pop(WithPos<Nibble>),
    Call(WithPos<Label>),
    Return,
    Add(WithPos<Nibble>),
    Rotate {
        shift: WithPos<Nibble>,
        register: WithPos<Nibble>,
    },

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
    Swap(WithPos<Nibble>),
    Sub(WithPos<Nibble>),
    Write(WithPos<Nibble>),
    WritePop(WithPos<Nibble>),
    And(WithPos<Nibble>),
    Nand(WithPos<Nibble>),
    Or(WithPos<Nibble>),
    Nor(WithPos<Nibble>),
    Xor(WithPos<Nibble>),
    NXor(WithPos<Nibble>),
    RegToFlags(WithPos<Nibble>),
    Compare(WithPos<Nibble>),
    SwapAdd(WithPos<Nibble>),
    SwapSub(WithPos<Nibble>),
    AddWithCarry(WithPos<Nibble>),
    SubWithCarry(WithPos<Nibble>),

    RawRamCall,
    Input,
    Output(Vec<OctDigit>),
}

#[derive(Debug, Clone)]
pub enum Meta {
    RomPage(WithPos<Nibble>),
    RamPage,
    Label(WithPos<Label>),
    UseFlags,
    Comment(WithPos<String>),
}

#[derive(Debug, Clone)]
pub enum Line {
    Command(Command),
    Meta(Meta),
}

#[derive(Debug, Clone)]
pub struct Assembly {
    lines: Vec<WithPos<Line>>,
}

impl Assembly {
    pub fn lines(&self) -> Vec<&Line> {
        self.lines.iter().map(|line| &line.t).collect::<Vec<_>>()
    }

    pub fn lines_with_pos(&self) -> Vec<&WithPos<Line>> {
        self.lines.iter().collect()
    }

    pub fn line_with_pos(&self, line: usize) -> &WithPos<Line> {
        &self.lines[line]
    }

    fn new(lines: Vec<WithPos<Line>>) -> Self {
        Self { lines }
    }
}

pub fn load_assembly(
    source: &str,
) -> Result<Assembly, lalrpop_util::ParseError<usize, Token<'_>, &'static str>> {
    assembly_grammar::AssemblyParser::new().parse(source)
}

pub mod assembly;
pub mod compile;
pub mod memory;
pub mod simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OctDigit {
    O0,
    O1,
    O2,
    O3,
    O4,
    O5,
    O6,
    O7,
}
impl OctDigit {
    pub fn as_u8(self) -> u8 {
        match self {
            OctDigit::O0 => 0,
            OctDigit::O1 => 1,
            OctDigit::O2 => 2,
            OctDigit::O3 => 3,
            OctDigit::O4 => 4,
            OctDigit::O5 => 5,
            OctDigit::O6 => 6,
            OctDigit::O7 => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nibble {
    N0,
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    N10,
    N11,
    N12,
    N13,
    N14,
    N15,
}
impl Nibble {
    pub fn as_u8(&self) -> u8 {
        self.as_usize() as u8
    }
    pub fn as_usize(&self) -> usize {
        match self {
            Nibble::N0 => 0,
            Nibble::N1 => 1,
            Nibble::N2 => 2,
            Nibble::N3 => 3,
            Nibble::N4 => 4,
            Nibble::N5 => 5,
            Nibble::N6 => 6,
            Nibble::N7 => 7,
            Nibble::N8 => 8,
            Nibble::N9 => 9,
            Nibble::N10 => 10,
            Nibble::N11 => 11,
            Nibble::N12 => 12,
            Nibble::N13 => 13,
            Nibble::N14 => 14,
            Nibble::N15 => 15,
        }
    }
}

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "1");

    let result = assembly::assembly_grammar::ProgramParser::new().parse(
        r#"
..ROM 0
VALUE 0
NOT
NOT
BRANCH Z foo
VALUE 0
JUMP bar
.LABEL foo
VALUE 1
.LABEL bar
RETURN
    "#,
    );

    println!("{:#?}", result);

    let result = result.unwrap();
    let mem = result.compile();
    mem.pprint();
    println!("{:?}", mem.run());

    // println!("{:?}", result);

    // nibblecode::do_shit();
}

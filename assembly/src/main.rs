use std::{thread::sleep, time::Duration};

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
    pub fn new(oct: u8) -> Self {
        match oct {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            3 => Self::O3,
            4 => Self::O4,
            5 => Self::O5,
            6 => Self::O6,
            7 => Self::O7,
            _ => {
                panic!()
            }
        }
    }
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

    let source = "\
..ROM 0
CALL run
RETURN

..ROM 1
.LABEL run
VALUE 1
DUP
POP %0
.LABEL loop
DUP
OUTPUT 1.0
SADD %0
.USEFLAGS
BRANCH C end
JUMP loop
.LABEL end
RETURN";
    println!("===Source===");
    println!("{source}");
    println!();

    let result = assembly::assembly_grammar::ProgramParser::new().parse(source);
    let result = result.unwrap();

    let mem = result.compile();
    println!("===Memory===");
    mem.pprint();
    println!();

    let file = std::fs::File::create("../memory.json").unwrap();
    serde_json::to_writer(file, &mem.to_json()).unwrap();

    let mut sim = mem.simulator();
    sim.subscribe_to_output(Box::new(|addr, value| {
        println!("{:?} {:?}", addr, value);
    }));
    let input = sim.input();
    std::thread::spawn(move || {
        sleep(Duration::from_millis(1000));
        input.lock().unwrap().push(3);
        sleep(Duration::from_millis(1000));
        input.lock().unwrap().push(5);
    });

    println!("===Execute===");
    println!("{:?}", sim.run(false, false));

    // println!("{:?}", result);

    // nibblecode::do_shit();
}

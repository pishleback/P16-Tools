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
    pub fn new(x: u8) -> Option<Self> {
        match x {
            0 => Some(Self::N0),
            1 => Some(Self::N1),
            2 => Some(Self::N2),
            3 => Some(Self::N3),
            4 => Some(Self::N4),
            5 => Some(Self::N5),
            6 => Some(Self::N6),
            7 => Some(Self::N7),
            8 => Some(Self::N8),
            9 => Some(Self::N9),
            10 => Some(Self::N10),
            11 => Some(Self::N11),
            12 => Some(Self::N12),
            13 => Some(Self::N13),
            14 => Some(Self::N14),
            15 => Some(Self::N15),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.as_usize() as u8
    }

    pub fn as_u16(&self) -> u16 {
        self.as_u8() as u16
    }

    pub fn as_u32(&self) -> u32 {
        self.as_u8() as u32
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

    pub fn hex_str(&self) -> &'static str {
        match self {
            Nibble::N0 => "0",
            Nibble::N1 => "1",
            Nibble::N2 => "2",
            Nibble::N3 => "3",
            Nibble::N4 => "4",
            Nibble::N5 => "5",
            Nibble::N6 => "6",
            Nibble::N7 => "7",
            Nibble::N8 => "8",
            Nibble::N9 => "9",
            Nibble::N10 => "A",
            Nibble::N11 => "B",
            Nibble::N12 => "C",
            Nibble::N13 => "D",
            Nibble::N14 => "E",
            Nibble::N15 => "F",
        }
    }
}

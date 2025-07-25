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
    pub fn new(x: u8) -> Result<Self, ()> {
        match x {
            0 => Ok(Self::N0),
            1 => Ok(Self::N1),
            2 => Ok(Self::N2),
            3 => Ok(Self::N3),
            4 => Ok(Self::N4),
            5 => Ok(Self::N5),
            6 => Ok(Self::N6),
            7 => Ok(Self::N7),
            8 => Ok(Self::N8),
            9 => Ok(Self::N9),
            10 => Ok(Self::N10),
            11 => Ok(Self::N11),
            12 => Ok(Self::N12),
            13 => Ok(Self::N13),
            14 => Ok(Self::N14),
            15 => Ok(Self::N15),
            _ => Err(()),
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

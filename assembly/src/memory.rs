#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Nibble {
    n: u8, // Use least sig 4 bits
}
impl Nibble {
    pub fn new(n: u8) -> Self {
        debug_assert!(n < 16);
        Self { n }
    }
    pub fn as_u8(&self) -> u8 {
        self.n as u8
    }
    pub fn as_u16(&self) -> u16 {
        self.n as u16
    }
    pub fn as_u32(&self) -> u32 {
        self.n as u32
    }
    pub fn as_usize(&self) -> usize {
        self.n as usize
    }
    pub fn as_enum(&self) -> super::Nibble {
        match self.n {
            0 => super::Nibble::N0,
            1 => super::Nibble::N1,
            2 => super::Nibble::N2,
            3 => super::Nibble::N3,
            4 => super::Nibble::N4,
            5 => super::Nibble::N5,
            6 => super::Nibble::N6,
            7 => super::Nibble::N7,
            8 => super::Nibble::N8,
            9 => super::Nibble::N9,
            10 => super::Nibble::N10,
            11 => super::Nibble::N11,
            12 => super::Nibble::N12,
            13 => super::Nibble::N13,
            14 => super::Nibble::N14,
            15 => super::Nibble::N15,
            _ => {
                unreachable!()
            }
        }
    }
    pub fn hex_str(&self) -> &'static str {
        &[
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
        ][self.n as usize]
    }
}

#[derive(Debug, Clone)]
pub struct RomPage {
    data: [Nibble; 256],
}
impl RomPage {
    fn zeros() -> Self {
        Self {
            data: core::array::from_fn(|_i| Nibble::new(0)),
        }
    }
    pub fn get_nibble(&self, ptr: u8) -> Nibble {
        self.data[ptr as usize]
    }
}

#[derive(Debug, Clone)]
pub struct RamMem {
    data: [u16; 4096],
}
impl RamMem {
    fn zeros() -> Self {
        Self { data: [0; 4096] }
    }
    pub fn get_value(&self, addr: u16) -> u16 {
        self.data[(addr % 4096) as usize]
    }
}

#[derive(Debug, Clone)]
pub struct Memory {
    rom: [RomPage; 16],
    ram: RamMem,
}
impl Memory {
    pub fn ram(&self) -> &RamMem {
        &self.ram
    }

    pub fn new(rom: [[Nibble; 256]; 16], ram: [Nibble; 1 << (12 + 2)]) -> Self {
        Self {
            rom: core::array::from_fn(|i| RomPage {
                data: core::array::from_fn(|j| rom[i][j]),
            }),
            ram: RamMem {
                data: core::array::from_fn(|i| {
                    let j = 4 * i;
                    ram[j].as_u16()
                        | (ram[j + 1].as_u16() << 4)
                        | (ram[j + 2].as_u16() << 8)
                        | (ram[j + 3].as_u16() << 12)
                }),
            },
        }
    }

    pub fn zeros() -> Self {
        let rom = core::array::from_fn(|_i| RomPage::zeros());
        let ram = RamMem::zeros();
        Self { rom, ram }
    }

    pub fn rom_page(&self, nibble: Nibble) -> &RomPage {
        &self.rom[nibble.as_usize()]
    }

    pub fn pprint(&self) {
        'ROMLOOP: for (n, rom_page) in self.rom.iter().enumerate() {
            let mut vals: Vec<Nibble> = rom_page.data.iter().map(|n| *n).collect();
            loop {
                match vals.last() {
                    Some(n) => {
                        if *n == Nibble::new(0) {
                            vals.pop().unwrap();
                            continue;
                        } else {
                            break;
                        }
                    }
                    None => {
                        continue 'ROMLOOP;
                    }
                }
            }

            print!("ROM {}: ", Nibble::new(n as u8).hex_str());
            for x in &vals {
                print!("{}", x.hex_str());
            }
            println!();
        }

        let mut vals: Vec<Nibble> = self
            .ram
            .data
            .iter()
            .map(|v| {
                vec![
                    Nibble::new((v & 15) as u8),
                    Nibble::new(((v >> 4) & 15) as u8),
                    Nibble::new(((v >> 8) & 15) as u8),
                    Nibble::new(((v >> 12) & 15) as u8),
                ]
            })
            .flatten()
            .collect();
        'RAMBLOCK: {
            loop {
                match vals.last() {
                    Some(n) => {
                        if *n == Nibble::new(0) {
                            vals.pop().unwrap();
                            continue;
                        } else {
                            break;
                        }
                    }
                    None => {
                        break 'RAMBLOCK;
                    }
                }
            }
            let mut i = 0usize;
            let mut j = 0usize;
            for n in vals {
                if i == 256 {
                    i = 0;
                    j += 1;
                    println!()
                }
                if i == 0 {
                    if j == 0 {
                        print!("RAM  : ");
                    } else {
                        print!("       ")
                    }
                }
                print!("{}", n.hex_str());
                i += 1;
            }
            println!();
        }
    }
}

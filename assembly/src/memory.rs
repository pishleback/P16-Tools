use crate::datatypes::Nibble;

#[derive(Debug, Clone)]
pub struct ProgramPage {
    data: [Nibble; 256],
}
impl ProgramPage {
    fn zeros() -> Self {
        Self {
            data: core::array::from_fn(|_i| Nibble::N0),
        }
    }

    pub fn get_nibble(&self, ptr: u8) -> Nibble {
        self.data[ptr as usize]
    }

    pub fn nibbles(&self) -> Vec<Nibble> {
        self.data.to_vec()
    }
}

// max allowed = 2^16
// max supported by in-game CPU = 2^12
pub const RAM_SIZE: u32 = 4096;
pub const RAM_SIZE_NIBBLES: u32 = RAM_SIZE * 4;

#[derive(Debug, Clone)]
pub struct RamMem {
    data: [u16; RAM_SIZE as usize],
}
impl RamMem {
    fn zeros() -> Self {
        Self {
            data: [0; RAM_SIZE as usize],
        }
    }

    pub fn read(&self, addr: u16) -> u16 {
        self.data[(addr % RAM_SIZE as u16) as usize]
    }

    pub fn write(&mut self, addr: u16, value: u16) {
        self.data[(addr % RAM_SIZE as u16) as usize] = value
    }

    pub fn data(&self) -> &[u16; RAM_SIZE as usize] {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct ProgramMemory {
    rom: [ProgramPage; 16],
    ram: RamMem,
}
impl ProgramMemory {
    pub fn ram(&self) -> &RamMem {
        &self.ram
    }

    pub fn ram_mut(&mut self) -> &mut RamMem {
        &mut self.ram
    }

    pub fn rom_page(&self, nibble: Nibble) -> &ProgramPage {
        &self.rom[nibble.as_usize()]
    }

    pub fn ram_page(&self, start: u16) -> ProgramPage {
        ProgramPage {
            data: std::array::from_fn(|i| {
                debug_assert!(i < 256);
                let i = i as u8;
                Nibble::new(
                    ((self.ram.read(start.wrapping_add(i as u16 / 4)) >> (4 * (3 - (i % 4)))) % 16)
                        as u8,
                )
                .unwrap()
            }),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        json.insert(
            "rom".to_string(),
            serde_json::Value::Array(
                self.rom
                    .iter()
                    .map(|page| {
                        serde_json::Value::Array(
                            if let Some(i) = page.data.iter().rposition(|&x| x != Nibble::N0) {
                                page.data[0..=i]
                                    .iter()
                                    .map(|n| serde_json::Value::Number(n.as_u8().into()))
                                    .collect()
                            } else {
                                [].to_vec()
                            },
                        )
                    })
                    .collect(),
            ),
        );
        if let Some(i) = self.ram.data.iter().rposition(|&x| x != 0) {
            json.insert(
                "ram".to_string(),
                serde_json::Value::Array(
                    self.ram.data[0..=i]
                        .iter()
                        .map(|v| serde_json::Value::Number((*v).into()))
                        .collect(),
                ),
            );
        }
        serde_json::Value::Object(json)
    }

    pub fn new(rom: [[Nibble; 256]; 16], ram: [Nibble; 1 << (12 + 2)]) -> Self {
        Self {
            rom: core::array::from_fn(|i| ProgramPage {
                data: core::array::from_fn(|j| rom[i][j]),
            }),
            ram: RamMem {
                data: core::array::from_fn(|i| {
                    let j = 4 * i;
                    ram[j + 3].as_u16()
                        | (ram[j + 2].as_u16() << 4)
                        | (ram[j + 1].as_u16() << 8)
                        | (ram[j].as_u16() << 12)
                }),
            },
        }
    }

    pub fn zeros() -> Self {
        let rom = core::array::from_fn(|_i| ProgramPage::zeros());
        let ram = RamMem::zeros();
        Self { rom, ram }
    }

    pub fn pprint(&self) {
        'ROMLOOP: for (n, rom_page) in self.rom.iter().enumerate() {
            let mut vals: Vec<Nibble> = rom_page.data.to_vec();
            loop {
                match vals.last() {
                    Some(n) => {
                        if *n == Nibble::N0 {
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

            print!("ROM {}: ", Nibble::new(n as u8).unwrap().hex_str());
            for x in &vals {
                print!("{}", x.hex_str());
            }
            println!();
        }

        let mut vals: Vec<Nibble> = self
            .ram
            .data
            .iter()
            .flat_map(|v| {
                vec![
                    Nibble::new((v & 15) as u8).unwrap(),
                    Nibble::new(((v >> 4) & 15) as u8).unwrap(),
                    Nibble::new(((v >> 8) & 15) as u8).unwrap(),
                    Nibble::new(((v >> 12) & 15) as u8).unwrap(),
                ]
            })
            .collect();
        'RAMBLOCK: {
            loop {
                match vals.last() {
                    Some(n) => {
                        if *n == Nibble::N0 {
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

pub use assembly::Nibble;
use mcschem::Block as PlainBlock;
use std::{collections::HashMap, str::FromStr};

pub enum Block {
    Plain(PlainBlock),
    Barrel { ss: Nibble },
}

pub struct Schem {
    blocks: HashMap<(i16, i16, i16), Block>,
}

impl Schem {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    pub fn place(&mut self, pos: (i16, i16, i16), block: Block) {
        assert!(self.blocks.insert(pos, block).is_none());
    }

    #[allow(clippy::result_unit_err)]
    pub fn finish<W: std::io::Write>(self, writer: &mut W) -> Result<(), ()> {
        let min_x = self.blocks.iter().map(|((x, _, _), _)| *x).min().unwrap();
        let max_x = self.blocks.iter().map(|((x, _, _), _)| *x).max().unwrap();
        let size_x = max_x - min_x + 1;

        let min_y = self.blocks.iter().map(|((_, y, _), _)| *y).min().unwrap();
        let max_y = self.blocks.iter().map(|((_, y, _), _)| *y).max().unwrap();
        let size_y = max_y - min_y + 1;

        let min_z = self.blocks.iter().map(|((_, _, z), _)| *z).min().unwrap();
        let max_z = self.blocks.iter().map(|((_, _, z), _)| *z).max().unwrap();
        let size_z = max_z - min_z + 1;

        let mut schem = mcschem::Schematic::new(
            mcschem::data_version::MC_1_18_2,
            size_x as u16,
            size_y as u16,
            size_z as u16,
        );
        for ((x, y, z), block) in self.blocks {
            let (x, y, z) = (
                (x - min_x) as usize,
                (y - min_y) as usize,
                (z - min_z) as usize,
            );
            match block {
                Block::Plain(block) => {
                    schem.set_block(x, y, z, block);
                }

                Block::Barrel { ss } => {
                    if ss == Nibble::N0 {
                        schem.set_block(
                            x,
                            y,
                            z,
                            mcschem::Block::from_str("minecraft:barrel[facing=up,open=false]")
                                .unwrap(),
                        );
                    } else {
                        schem.set_block_entity(
                            x,
                            y,
                            z,
                            mcschem::Block::from_str("minecraft:barrel[facing=up,open=false]")
                                .unwrap(),
                            mcschem::BlockEntity::Barrel {
                                items: mcschem::utils::barrel_ss(ss.as_usize()),
                            },
                        );
                    }
                }
            };
        }
        schem
            .export(writer, (min_x as i32, min_y as i32, min_z as i32))
            .map_err(|_| ())
    }
}

impl Schem {
    fn make_torch_rom_page(&mut self, ox: i16, oy: i16, oz: i16, nibbles: Vec<Nibble>) {
        assert_eq!(nibbles.len(), 256);
        fn set_nibble(schem: &mut Schem, x: i16, y: i16, z: i16, n: Nibble) {
            for i in 0usize..4 {
                let dx = -2 * i as i16;
                let block = if n.as_usize() & (1 << (3 - i)) != 0 {
                    Block::Plain(
                        PlainBlock::from_str("minecraft:redstone_wall_torch[facing=north]")
                            .unwrap(),
                    )
                } else {
                    Block::Plain(PlainBlock::from_str("minecraft:glass").unwrap())
                };
                schem.place((x + dx, y, z), block);
            }
        }

        for (i, n) in nibbles.iter().enumerate() {
            let (q, r) = (i / 32, i % 32);
            set_nibble(self, ox - 8 * q as i16, oy, oz - 2 * r as i16, *n);
        }
    }

    fn make_barrel_rom_page(&mut self, ox: i16, oy: i16, oz: i16, nibbles: Vec<Nibble>) {
        assert_eq!(nibbles.len(), 256);
        for a in 0usize..8 {
            for d in 0usize..32 {
                let pos = (ox - 2 * d as i16, oy - 2 * a as i16, oz);
                let ss = nibbles[d + 32 * a];
                if ss == Nibble::N0 {
                    self.place(
                        pos,
                        Block::Plain(PlainBlock::from_str("minecraft:glass").unwrap()),
                    );
                } else {
                    self.place(pos, Block::Barrel { ss });
                }
            }
        }
    }

    pub fn place_rom_page(&mut self, page: Nibble, memory: &assembly::ProgramPage) {
        let page = page.as_usize();
        match page {
            0 => {
                println!("Schematics for ROM page 0 are not supported.");
            }
            1..=3 => {
                self.make_torch_rom_page(-5, -10 - 5 * (page as i16 - 1), -5, memory.nibbles());
            }
            4..=15 => {
                self.make_barrel_rom_page(
                    -13,
                    -11 - if page.is_multiple_of(2) { 16 } else { 0 },
                    13 + 4 * ((page as i16 - 4) / 2),
                    memory.nibbles(),
                );
            }
            _ => {
                panic!("Invalid ROM page {}", page);
            }
        }
    }
}

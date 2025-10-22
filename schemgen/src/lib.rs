use mcschem::Block as PlainBlock;
use std::{collections::HashMap, str::FromStr};

pub enum Block {
    Plain(PlainBlock),
    Barrel(usize),
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

    pub fn finish<W: std::io::Write>(self, writer: &mut W) {
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

                Block::Barrel(ss) => {
                    if ss == 0 {
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
                                items: mcschem::utils::barrel_ss(ss),
                            },
                        );
                    }
                }
            };
        }
        schem
            .export(writer, (min_x as i32, min_y as i32, min_z as i32))
            .unwrap();
    }
}

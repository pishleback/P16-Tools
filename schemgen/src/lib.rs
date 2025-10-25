pub use assembly::Nibble;
use mcschem::Block as PlainBlock;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mat4 {
    entries: [[i16; 4]; 4],
}

impl Mat4 {
    fn print_rows(&self) {
        for row in self.entries {
            println!("{:?}", row);
        }
    }

    fn identity() -> Self {
        Self {
            entries: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],
        }
    }

    fn apply(&self, vec: [i16; 4]) -> [i16; 4] {
        std::array::from_fn(|r| (0usize..4).map(|c| self.entries[r][c] * vec[c]).sum())
    }
}

impl std::ops::Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, other: Mat4) -> Self::Output {
        Mat4 {
            entries: std::array::from_fn(|r| {
                std::array::from_fn(|c| {
                    (0usize..4)
                        .map(|k| self.entries[r][k] * other.entries[k][c])
                        .sum()
                })
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Compass {
    North,
    East,
    South,
    West,
}

impl Compass {
    // To (dx, dy) of length 1 pointing in the direction of self
    fn to_vec(self) -> (i16, i16) {
        match self {
            Compass::North => (0, -1),
            Compass::East => (1, 0),
            Compass::South => (0, 1),
            Compass::West => (-1, 0),
        }
    }

    // From (dx, dy) of length 1 pointing in the direction of self
    fn from_vec(vec: (i16, i16)) -> Self {
        match vec {
            (0, -1) => Self::North,
            (1, 0) => Self::East,
            (0, 1) => Self::South,
            (-1, 0) => Self::West,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Transform {
    // Bottom row is 0 0 0 1
    // Act on positions by (x, y, z, 1) -> M (x, y, z, 1).012
    // Act on vectors by (x, y, z, 0) -> M (x, y, z, 0).012
    forward: Mat4,
    backward: Mat4,
}

impl std::ops::Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(self, other: Transform) -> Self::Output {
        Transform::new(self.forward * other.forward, other.backward * self.backward)
    }
}

impl Transform {
    fn new(forward: Mat4, backward: Mat4) -> Self {
        debug_assert_eq!(forward * backward, Mat4::identity());
        Self { forward, backward }
    }

    fn inverse(self) -> Self {
        Self::new(self.backward, self.forward)
    }

    fn translate((dx, dy, dz): (i16, i16, i16)) -> Self {
        Self::new(
            Mat4 {
                entries: [[1, 0, 0, dx], [0, 1, 0, dy], [0, 0, 1, dz], [0, 0, 0, 1]],
            },
            Mat4 {
                entries: [[1, 0, 0, -dx], [0, 1, 0, -dy], [0, 0, 1, -dz], [0, 0, 0, 1]],
            },
        )
    }

    fn rotate() -> Self {
        Self::new(
            Mat4 {
                entries: [[0, 0, -1, 0], [0, 1, 0, 0], [1, 0, 0, 0], [0, 0, 0, 1]],
            },
            Mat4 {
                entries: [[0, 0, 1, 0], [0, 1, 0, 0], [-1, 0, 0, 0], [0, 0, 0, 1]],
            },
        )
    }

    fn identity() -> Self {
        Self::new(Mat4::identity(), Mat4::identity())
    }

    fn flip_x() -> Self {
        Self::new(
            Mat4 {
                entries: [[-1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],
            },
            Mat4 {
                entries: [[-1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],
            },
        )
    }

    fn flip_z() -> Self {
        Self::new(
            Mat4 {
                entries: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, -1, 0], [0, 0, 0, 1]],
            },
            Mat4 {
                entries: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, -1, 0], [0, 0, 0, 1]],
            },
        )
    }

    fn apply_pos(&self, pos: (i16, i16, i16)) -> (i16, i16, i16) {
        let out = self.forward.apply([pos.0, pos.1, pos.2, 1]);
        debug_assert_eq!(out[3], 1);
        (out[0], out[1], out[2])
    }

    fn apply_vec(&self, vec: (i16, i16, i16)) -> (i16, i16, i16) {
        let out = self.forward.apply([vec.0, vec.1, vec.2, 0]);
        debug_assert_eq!(out[3], 0);
        (out[0], out[1], out[2])
    }

    fn apply_compass(&self, compass: Compass) -> Compass {
        let vec = compass.to_vec();
        let vec = self.apply_vec((vec.0, 0, vec.1));
        assert_eq!(vec.1, 0);
        Compass::from_vec((vec.0, vec.2))
    }
}

struct Coords {
    // local -> global
    transform: Transform,
}

impl Coords {
    fn apply_global_transform(&mut self, transform: Transform) {
        self.transform = transform * self.transform;
    }

    fn apply_local_transform(&mut self, transform: Transform) {
        self.transform = self.transform * transform;
    }

    fn local_to_global_pos(&self, pos: (i16, i16, i16)) -> (i16, i16, i16) {
        self.transform.apply_pos(pos)
    }

    fn local_to_global_vec(&self, vec: (i16, i16, i16)) -> (i16, i16, i16) {
        self.transform.apply_vec(vec)
    }

    fn local_to_global_compass(&self, compass: Compass) -> Compass {
        self.transform.apply_compass(compass)
    }
}

#[derive(Debug, Clone)]
pub enum Block {
    Plain(PlainBlock),
    Barrel {
        ss: Nibble,
    },
    Dust {
        power: u16,
    },
    Torch {
        lit: bool,
    },
    WallTorch {
        lit: bool,
        facing: Compass,
    },
    Repeater {
        powered: bool,
        facing: Compass,
        delay: u16,
    },
}

pub struct Blocks {
    blocks: HashMap<(i16, i16, i16), Block>,
}

impl Blocks {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    pub fn place(&mut self, pos: (i16, i16, i16), block: &Block) {
        if self.blocks.insert(pos, block.clone()).is_some() {
            panic!("Pos {:?} taken.", pos);
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn finish<W: std::io::Write>(self, writer: &mut W) -> Result<(), ()> {
        if self.blocks.is_empty() {
            println!("No blocks!");
            return Err(());
        }

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

                Block::Dust { power } => {
                    schem.set_block(
                        x,
                        y,
                        z,
                        mcschem::Block::from_str(
                            format!("minecraft:redstone_wire[power={power}]").as_str(),
                        )
                        .unwrap(),
                    );
                }

                Block::Torch { lit } => {
                    schem.set_block(
                        x,
                        y,
                        z,
                        mcschem::Block::from_str(
                            format!("minecraft:redstone_torch[lit={lit}]").as_str(),
                        )
                        .unwrap(),
                    );
                }

                Block::WallTorch { lit, facing } => {
                    schem.set_block(
                        x,
                        y,
                        z,
                        mcschem::Block::from_str(
                            format!(
                                "minecraft:redstone_wall_torch[lit={lit},facing={}]",
                                match facing {
                                    Compass::North => "north",
                                    Compass::East => "east",
                                    Compass::South => "south",
                                    Compass::West => "west",
                                }
                            )
                            .as_str(),
                        )
                        .unwrap(),
                    );
                }

                Block::Repeater {
                    powered,
                    facing,
                    delay,
                } => {
                    schem.set_block(
                        x,
                        y,
                        z,
                        mcschem::Block::from_str(
                            format!(
                                "minecraft:repeater[facing={},powered={powered},delay={delay}]",
                                match facing {
                                    Compass::North => "south",
                                    Compass::East => "west",
                                    Compass::South => "north",
                                    Compass::West => "east",
                                }
                            )
                            .as_str(),
                        )
                        .unwrap(),
                    );
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

impl Blocks {
    fn make_torch_rom_page(&mut self, ox: i16, oy: i16, oz: i16, nibbles: Vec<Nibble>) {
        assert_eq!(nibbles.len(), 256);
        fn set_nibble(schem: &mut Blocks, x: i16, y: i16, z: i16, n: Nibble) {
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
                schem.place((x + dx, y, z), &block);
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
                        &Block::Plain(PlainBlock::from_str("minecraft:glass").unwrap()),
                    );
                } else {
                    self.place(pos, &Block::Barrel { ss });
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

struct RamCard {
    coords: Coords,
    section_sizes: Vec<usize>,
    data_block: Block,
    read_block: Block,
}

impl RamCard {
    // first: The block at the very end where the input logic is
    // aligned: The blocks above the output lines
    // between: The blocks between the output lines
    // join: The blocks between sections of output lines
    fn place_stacked(
        &self,
        schem: &mut Blocks,
        offset: (i16, i16, i16),
        first: Option<&Block>,
        aligned: Option<&Block>,
        between: Option<&Block>,
        join: Option<&Block>,
    ) {
        if let Some(first) = first {
            schem.place(self.coords.local_to_global_pos(offset), first);
        }
        if let Some(aligned) = aligned {
            let mut dz = 2i16;
            for &size in &self.section_sizes {
                for _ in 0..size {
                    schem.place(
                        self.coords
                            .local_to_global_pos((offset.0, offset.1, offset.2 + dz)),
                        aligned,
                    );
                    dz += 2;
                }
            }
        }
        if let Some(between) = between {
            let mut dz = 3i16;
            for &size in &self.section_sizes {
                for _ in 1..size {
                    schem.place(
                        self.coords
                            .local_to_global_pos((offset.0, offset.1, offset.2 + dz)),
                        between,
                    );
                    dz += 2;
                }
                dz += 2;
            }
        }
        if let Some(join) = join {
            let mut dz = 1i16;
            for &size in &self.section_sizes {
                schem.place(
                    self.coords
                        .local_to_global_pos((offset.0, offset.1, offset.2 + dz)),
                    join,
                );
                dz += 2 * size as i16;
            }
        }
    }

    fn place_data(&mut self, schem: &mut Blocks, data: Vec<Vec<bool>>, first: bool, last: bool) {
        let n = self.section_sizes.len();
        assert_eq!(n, data.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            assert_eq!(self.section_sizes[i], data[i].len());
        }

        // Read lines
        self.place_stacked(schem, (0, 0, 0), Some(&self.read_block), None, None, None);
        self.place_stacked(
            schem,
            (0, 1, 0),
            Some(&Block::Repeater {
                powered: true,
                facing: self.coords.local_to_global_compass(Compass::East),
                delay: 3,
            }),
            None,
            None,
            None,
        );
        self.place_stacked(
            schem,
            (1, 0, 0),
            Some(&self.read_block),
            Some(&self.read_block),
            Some(&self.read_block),
            Some(&self.read_block),
        );
        self.place_stacked(
            schem,
            (1, 1, 0),
            Some(&Block::Dust { power: 15 }),
            Some(&Block::Dust { power: 15 }),
            Some(&Block::Dust { power: 15 }),
            Some(&Block::Repeater {
                powered: true,
                facing: self.coords.local_to_global_compass(Compass::South),
                delay: 1,
            }),
        );
        // The torches for the data
        {
            let mut dz = 2i16;
            for (i, &size) in self.section_sizes.iter().enumerate() {
                for j in 0usize..size {
                    if data[i][j] {
                        schem.place(
                            self.coords.local_to_global_pos((0, 0, dz)),
                            &Block::WallTorch {
                                lit: false,
                                facing: self.coords.local_to_global_compass(Compass::West),
                            },
                        );
                    }
                    dz += 2;
                }
            }
        }

        // Data lines
        self.place_stacked(schem, (0, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(schem, (1, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(
            schem,
            (0, -1, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );
        self.place_stacked(
            schem,
            (1, -1, 0),
            None,
            Some(&Block::Repeater {
                powered: false,
                facing: self.coords.local_to_global_compass(Compass::West),
                delay: 3,
            }),
            None,
            None,
        );

        // Update coords
        self.coords
            .apply_local_transform(Transform::translate((2, 0, 0)));
    }

    fn place_new_layer(&mut self, schem: &mut Blocks) {
        // Read lines
        schem.place(self.coords.local_to_global_pos((0, 1, 0)), &self.read_block);
        schem.place(
            self.coords.local_to_global_pos((0, 2, 0)),
            &Block::Dust { power: 15 },
        );
        schem.place(self.coords.local_to_global_pos((1, 2, 0)), &self.read_block);
        schem.place(
            self.coords.local_to_global_pos((1, 3, 0)),
            &Block::Torch { lit: false },
        );
        schem.place(self.coords.local_to_global_pos((1, 4, 0)), &self.read_block);
        schem.place(
            self.coords.local_to_global_pos((1, 5, 0)),
            &Block::Torch { lit: true },
        );

        // Data lines
        self.place_stacked(schem, (0, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(schem, (0, 0, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(schem, (1, -1, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(schem, (2, 0, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(schem, (1, 1, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(
            schem,
            (0, -1, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );
        self.place_stacked(
            schem,
            (1, 0, 0),
            None,
            Some(&Block::Repeater {
                powered: false,
                facing: self.coords.local_to_global_compass(Compass::West),
                delay: 1,
            }),
            None,
            None,
        );
        self.place_stacked(
            schem,
            (2, 1, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );
        self.place_stacked(
            schem,
            (1, 2, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );

        self.coords
            .apply_local_transform(Transform::flip_x() * Transform::translate((0, 4, 0)));
    }

    fn place_start(&mut self, schem: &mut Blocks) {
        self.place_stacked(schem, (-1, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(
            schem,
            (-1, -1, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );
        self.place_stacked(schem, (-2, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(
            schem,
            (-2, -1, 0),
            None,
            Some(&Block::Dust { power: 0 }),
            None,
            None,
        );
        self.place_stacked(schem, (-3, -2, 0), None, Some(&self.data_block), None, None);
        self.place_stacked(
            schem,
            (-3, -1, 0),
            None,
            Some(&Block::Repeater {
                powered: false,
                facing: self.coords.local_to_global_compass(Compass::West),
                delay: 1,
            }),
            None,
            None,
        );

        self.place_stacked(schem, (-2, 1, 0), Some(&self.read_block), None, None, None);
        self.place_stacked(schem, (-3, 0, 0), Some(&self.read_block), None, None, None);
        self.place_stacked(
            schem,
            (-1, 1, 0),
            Some(&Block::WallTorch {
                lit: true,
                facing: Compass::East,
            }),
            None,
            None,
            None,
        );
        self.place_stacked(
            schem,
            (-3, 1, 0),
            Some(&Block::Repeater {
                powered: false,
                facing: self.coords.local_to_global_compass(Compass::East),
                delay: 1,
            }),
            None,
            None,
            None,
        );
    }
}

impl Blocks {
    // input is a list of (addr, value) pairs to write
    pub fn place_ram_data(&mut self, values: Vec<(u16, u16)>) {
        println!("{:?}", values);

        let mut state = RamCard {
            coords: Coords {
                transform: Transform::translate((47, -49, -78)),
            },
            section_sizes: vec![8, 6, 8, 8],
            data_block: Block::Plain(PlainBlock::from_str("minecraft:gray_wool").unwrap()),
            read_block: Block::Plain(PlainBlock::from_str("minecraft:lime_wool").unwrap()),
        };

        state.place_start(self);

        let mut i = 0;
        let layer_at_i = 8;
        for (addr, value) in values {
            // Data
            {
                if i == layer_at_i {
                    i = 0;
                    state.place_new_layer(self);
                }
                state.place_data(
                    self,
                    vec![
                        (0..8).map(|i| (addr >> i) & 1 != 0).collect(),
                        (8..12)
                            .map(|i| (addr >> i) & 1 != 0)
                            .chain(vec![false, true])
                            .collect(),
                        (0..8).map(|i| (value >> i) & 1 != 0).collect(),
                        (8..16).map(|i| (value >> i) & 1 != 0).collect(),
                    ],
                    i == 0,
                    i == layer_at_i - 1,
                );
                i += 1;
            }
            {
                // Dummy for more delay
                if i == layer_at_i {
                    i = 0;
                    state.place_new_layer(self);
                }
                state.place_data(
                    self,
                    vec![
                        (0..8).map(|_| false).collect(),
                        (8..12).map(|_| false).chain(vec![false, false]).collect(),
                        (0..8).map(|_| false).collect(),
                        (8..16).map(|_| false).collect(),
                    ],
                    i == 0,
                    i == layer_at_i - 1,
                );
                i += 1;
            }
        }
    }
}

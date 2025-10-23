#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::nursery,
    clippy::suspicious,
    clippy::style
)]
#![allow(clippy::semicolon_inside_block, clippy::just_underscores_and_digits)]

use quartz_nbt as nbt;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io;
use std::str::FromStr;

pub mod data_version;
pub mod utils;

/// A struct holding infomation about a schematic
#[derive(Debug, Clone)]
pub struct Schematic {
    data_version: i32,

    blocks: Vec<Block>,
    block_entities: HashMap<[u16; 3], BlockEntity>,
    size_x: u16,
    size_y: u16,
    size_z: u16,
}

/// A block with ID and properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    id: String,
    properties: BTreeMap<String, String>,
}

/// A block entity
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum BlockEntity {
    /// Represents a barrel
    Barrel { items: Vec<ItemSlot> },
    // /// A post-1.20 sign
    // Sign {
    // },
    /// A pre-1.20 sign
    SignPre1D20 {
        glowing: bool,
        color: String,
        line_1: String,
        line_2: String,
        line_3: String,
        line_4: String,
    },
}

/// An item slot in a container
#[derive(Debug, Clone)]
pub struct ItemSlot {
    pub id: String,
    pub extra: nbt::NbtCompound,
    pub count: i8,
    pub slot: i8,
}

impl FromStr for Block {
    type Err = ();
    fn from_str(block: &str) -> Result<Self, ()> {
        let (id, properties) = block
            .split_once('[')
            .map_or_else(|| (block, None), |(a, b)| (a, Some(b)));

        let mut prop = BTreeMap::new();
        if let Some(properties) = properties {
            if !matches!(properties.chars().last(), Some(']')) {
                return Err(());
            }

            let properties = &properties[..properties.len() - 1];

            for property in properties.split(',') {
                let (k, v) = property.split_once('=').ok_or(())?;
                prop.insert(k.to_string(), v.to_string());
            }
        }

        Ok(Self {
            id: id.to_string(),
            properties: prop,
        })
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.id.fmt(f)?;

        if !self.properties.is_empty() {
            write!(
                f,
                "[{}]",
                self.properties
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<String>>()
                    .join(",")
            )?;
        }

        Ok(())
    }
}

impl Schematic {
    /// Initialize a new schematic filled with `minecraft:air`
    pub fn new(data_version: i32, size_x: u16, size_y: u16, size_z: u16) -> Self {
        Self {
            data_version,
            blocks: vec![
                Block::from_str("minecraft:air").unwrap();
                size_x as usize * size_y as usize * size_z as usize
            ],
            block_entities: HashMap::new(),
            size_x,
            size_y,
            size_z,
        }
    }

    /// Sets a block in the schematic
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: Block) {
        if x >= self.size_x as usize || y >= self.size_y as usize || z >= self.size_z as usize {
            panic!("Set block to ({x}, {y}, {z}) which is out of bound");
        }

        self.blocks[y * (self.size_x * self.size_z) as usize + z * self.size_x as usize + x] =
            block;
    }

    /// Sets a block entity in the schematic
    pub fn set_block_entity(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: Block,
        be: BlockEntity,
    ) {
        if x >= self.size_x as usize || y >= self.size_y as usize || z >= self.size_z as usize {
            panic!("Set block to ({x}, {y}, {z}) which is out of bound");
        }

        self.blocks[y * (self.size_x * self.size_z) as usize + z * self.size_x as usize + x] =
            block;

        self.block_entities
            .insert([x as u16, y as u16, z as u16], be);
    }
    /// Export the schematic to a writer
    pub fn export<W: io::Write>(
        &self,
        writer: &mut W,
        offset: (i32, i32, i32),
    ) -> Result<(), quartz_nbt::io::NbtIoError> {
        let mut palette = Vec::new();
        let mut block_data = Vec::new();
        for block in self.blocks.iter() {
            if !palette.contains(block) {
                palette.push(block.clone());
            }

            let mut id = palette.iter().position(|v| v == block).unwrap();

            while id & 0x80 != 0 {
                block_data.push(id as u8 as i8 & 0x7F | 0x80_u8 as i8);
                id >>= 7;
            }
            block_data.push(id as u8 as i8);
        }

        let mut palette_nbt = nbt::NbtCompound::new();
        for (bi, b) in palette.iter().enumerate() {
            palette_nbt.insert(format!("{b}"), nbt::NbtTag::Int(bi as i32));
        }

        let mut block_entities = vec![];
        for (p, e) in self.block_entities.iter() {
            let mut compound = nbt::compound! {
                "Pos": [I; p[0] as i32, p[1] as i32, p[2] as i32],
                "Id": e.id()
            };
            e.add_data(&mut compound);
            block_entities.push(compound);
        }

        let schem = nbt::compound! {
            "Version": 2_i32,
            "DataVersion": self.data_version,
            "Metadata": nbt::compound! {
                "WEOffsetX": offset.0,
                "WEOffsetY": offset.1,
                "WEOffsetZ": offset.2,
                "MCSchematicMetadata": nbt::compound! {
                    "Generated": "Generated with rust crate `mcschem`"
                },
            },
            "Width": self.size_x as i16,
            "Height": self.size_y as i16,
            "Length": self.size_z as i16,
            "PaletteMax": palette.len() as i32,
            "Palette": palette_nbt,
            "BlockData": nbt::NbtTag::ByteArray(block_data),
            "BlockEntities": nbt::NbtList::from(block_entities),
        };

        // println!("{schem:#?}");

        nbt::io::write_nbt(
            writer,
            Some("Schematic"),
            &schem,
            nbt::io::Flavor::GzCompressed,
        )
    }
}

impl BlockEntity {
    fn id(&self) -> &'static str {
        match self {
            Self::Barrel { .. } => "minecraft:barrel",
            /* Self::Sign { .. } | */ Self::SignPre1D20 { .. } => "minecraft:sign",
        }
    }

    fn add_data(&self, compound: &mut nbt::NbtCompound) {
        match self {
            Self::Barrel { items } => {
                let mut items_nbt = Vec::with_capacity(items.len());

                for i in items.iter() {
                    items_nbt.push(i.to_compound());
                }

                compound.insert("Items", nbt::NbtList::from(items_nbt));
            }
            // Self::Sign {  } => {
            //     todo!();
            // },
            Self::SignPre1D20 {
                glowing,
                color,
                line_1,
                line_2,
                line_3,
                line_4,
            } => {
                compound.insert("GlowingText", *glowing as i8);
                compound.insert("Color", color.clone());
                compound.insert("Text1", line_1.clone());
                compound.insert("Text2", line_2.clone());
                compound.insert("Text3", line_3.clone());
                compound.insert("Text4", line_4.clone());
            }
        }
    }
}

impl ItemSlot {
    fn to_compound(&self) -> nbt::NbtCompound {
        nbt::compound! {
            "Count": self.count,
            "Slot": self.slot,
            "id": self.id.clone(),
            "tag": self.extra.clone()
        }
    }
}

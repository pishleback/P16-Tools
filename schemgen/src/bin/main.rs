use mcschem::Block as PlainBlock;
use schemgen::{Block, Schem};
use std::str::FromStr;

fn main() {
    let mut schem = Schem::new();

    schem.place(
        (-2, -1, -2),
        Block::Plain(PlainBlock::from_str("minecraft:dirt").unwrap()),
    );
    schem.place(
        (2, -1, 2),
        Block::Plain(PlainBlock::from_str("minecraft:dirt").unwrap()),
    );
    schem.place(
        (2, -1, -2),
        Block::Plain(PlainBlock::from_str("minecraft:stone").unwrap()),
    );
    schem.place(
        (-2, -1, 2),
        Block::Plain(PlainBlock::from_str("minecraft:glass").unwrap()),
    );
    schem.place(
        (0, -1, 0),
        Block::Barrel {
            ss: assembly::Nibble::N6,
        },
    );

    let mut file = std::fs::File::create("example.schem").unwrap();
    schem.finish(&mut file);
}

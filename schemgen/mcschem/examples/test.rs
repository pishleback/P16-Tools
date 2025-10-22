use std::str::FromStr;

fn main() {
    let mut schem = mcschem::Schematic::new(mcschem::data_version::MC_1_18_2, 1, 2, 1);

    schem.set_block_entity(
        0, 0, 0,
        mcschem::Block::from_str("minecraft:barrel[facing=north,open=false]").unwrap(),
        mcschem::BlockEntity::Barrel {
            items: mcschem::utils::barrel_ss(3)
        }
    );

    schem.set_block_entity(
        0, 1, 0,
        mcschem::Block::from_str("minecraft:oak_sign[rotation=8,waterlogged=false]").unwrap(),
        mcschem::BlockEntity::SignPre1D20 {
            glowing: true,
            color: "lime".to_string(),
            line_1: r#"["Line 1"]"#.to_string(),
            line_2: r#"["Line 2"]"#.to_string(),
            line_3: r#"["Line 3"]"#.to_string(),
            line_4: r#"["Line 4"]"#.to_string()
        }
    );

    let mut file = std::fs::File::create("schematic.schem").unwrap();
    schem.export(&mut file).unwrap();
}

use crate::app::simulator::SimulatorStateTrait;
use crate::app::state::State;
use assembly::ProgramPagePtr;
use assembly::{CompiledLine, FullCompileResult, Nibble};
use egui::{Color32, TextBuffer, TextFormat, Ui, Visuals, text::LayoutJob};
use schemgen::transforms::{Coords, Transform};
use schemgen::{Block, Blocks, Compass};
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
mod save_schem {
    use js_sys::Uint8Array;
    use schemgen::Blocks;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{Blob, BlobPropertyBag, Document, HtmlAnchorElement, Url, Window};

    /// Trigger a browser download of arbitrary binary data
    #[wasm_bindgen]
    pub fn download_binary_file(filename: &str, bytes: &[u8]) -> Result<(), JsValue> {
        // Get the window and document
        let window: Window = web_sys::window().ok_or("No global window exists")?;
        let document: Document = window.document().ok_or("No document on window")?;

        // Convert Rust &[u8] slice into a JS Uint8Array
        let uint8_array = Uint8Array::from(bytes);

        // Put it into a JS array, as Blob::new_with_u8_array_sequence expects an array of parts
        let parts = js_sys::Array::new();
        parts.push(&uint8_array);

        // Create a binary blob with MIME type
        let options = BlobPropertyBag::new();
        options.set_type("application/octet-stream");
        let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;

        // Create a temporary object URL for the blob
        let url = Url::create_object_url_with_blob(&blob)?;

        // Create an <a> element and click it to trigger download
        let a = document
            .create_element("a")?
            .dyn_into::<HtmlAnchorElement>()?;
        a.set_href(&url);
        a.set_download(filename);
        a.click();

        // Clean up
        Url::revoke_object_url(&url)?;

        Ok(())
    }

    pub fn save(schem: Blocks) {
        let mut bytes: Vec<u8> = vec![];
        schem.finish(&mut bytes).unwrap();
        download_binary_file("p16_program.schem", &bytes).unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod save_schem {
    use schemgen::Blocks;

    pub fn save(schem: Blocks) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save schematic as...")
            .save_file()
        {
            let mut file = std::fs::File::create(path).unwrap();
            if let Err(()) = schem.finish(&mut file) {
                println!("Failed :(");
            }
        }
    }
}

pub fn update(
    state: &mut State,
    compile_result: &FullCompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .stick_to_bottom(false)
        // .max_height(600.0)
        .show(ui, |ui| {
            if let Ok((Ok((Ok(compiled), _page_layout)), _assembly)) = &compile_result {
                let raw_memory = compiled.memory().clone();
                let partial_raw_memory = compiled.partial_memory().clone();

                if ui
                    .button({
                        #[cfg(target_arch = "wasm32")]
                        let s = "Download Schematic";
                        #[cfg(not(target_arch = "wasm32"))]
                        let s = "Save Schematic";
                        s
                    })
                    .clicked()
                {
                    let mut schem = schemgen::Blocks::new();
                    for i in 1u8..16 {
                        let i = Nibble::new(i).unwrap();
                        place_rom_page(&mut schem, i, raw_memory.rom_page(i));
                    }
                    place_ram_data(&mut schem, partial_raw_memory.ram());
                    save_schem::save(schem);
                }

                // Show ROM pages
                for rom_page in (0..16).map(|n| Nibble::new(n).unwrap()) {
                    let nibbles = raw_memory.rom_page(rom_page).nibbles();
                    let lines = compiled.rom_lines(rom_page);
                    if !lines.is_empty() {
                        egui::CollapsingHeader::new(format!("ROM {}", rom_page.hex_str())).show(
                            ui,
                            |ui| {
                                page(
                                    ui,
                                    nibbles,
                                    lines,
                                    state.selected_lines.as_ref().unwrap_or(&HashSet::new()),
                                    state.simulator.simulator().map(|s| s.get_pc()).and_then(
                                        |ptr| match ptr.page {
                                            ProgramPagePtr::Rom { page } => {
                                                if page == rom_page {
                                                    Some(ptr.counter)
                                                } else {
                                                    None
                                                }
                                            }
                                            ProgramPagePtr::Ram { .. } => None,
                                        },
                                    ),
                                );
                            },
                        );
                    }
                }

                // Show RAM pages
                for (ram_page_num, ram_page) in compiled.ram_pages().into_iter().enumerate() {
                    let live_nibbles = state
                        .simulator
                        .simulator()
                        .map(|s| s.get_memory())
                        .map(|m| m.ram_page(ram_page.start).nibbles());
                    let nibbles = raw_memory.ram_page(ram_page.start).nibbles();

                    if live_nibbles
                        .as_ref()
                        .map_or_else(|| true, |live_nibbles| nibbles == *live_nibbles)
                    {
                        let lines = compiled.ram_lines(ram_page_num);
                        if !lines.is_empty() {
                            egui::CollapsingHeader::new(format!("RAM {}", ram_page_num))
                                .id_salt(format!("RAM {}", ram_page_num))
                                .show(ui, |ui| {
                                    page(
                                        ui,
                                        nibbles,
                                        lines,
                                        state.selected_lines.as_ref().unwrap_or(&HashSet::new()),
                                        state.simulator.simulator().map(|s| s.get_pc()).and_then(
                                            |ptr| match ptr.page {
                                                ProgramPagePtr::Rom { .. } => None,
                                                ProgramPagePtr::Ram { addr } => {
                                                    if addr == ram_page.start {
                                                        Some(ptr.counter)
                                                    } else {
                                                        None
                                                    }
                                                }
                                            },
                                        ),
                                    );
                                });
                        }
                    } else {
                        egui::CollapsingHeader::new(format!("RAM {} (Modified)", ram_page_num))
                            .id_salt(format!("RAM {}", ram_page_num))
                            .show(ui, |ui| {
                                page_raw(
                                    ui,
                                    live_nibbles.unwrap(),
                                    state.simulator.simulator().map(|s| s.get_pc()).and_then(
                                        |ptr| match ptr.page {
                                            ProgramPagePtr::Rom { .. } => None,
                                            ProgramPagePtr::Ram { addr } => {
                                                if addr == ram_page.start {
                                                    Some(ptr.counter)
                                                } else {
                                                    None
                                                }
                                            }
                                        },
                                    ),
                                );
                            });
                    }
                }
            }
        });
}

fn page(
    ui: &mut Ui,
    nibbles: Vec<Nibble>,
    lines: &Vec<CompiledLine>,
    selected_assembly: &HashSet<usize>,
    pc: Option<u8>,
) {
    let mut nibbles = nibbles.iter().map(|n| n.hex_str()).collect::<String>();

    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job(text.as_str(), ui.visuals(), lines, selected_assembly, pc);
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

    fn layout_job(
        page: &str,
        visuals: &Visuals,
        lines: &Vec<CompiledLine>,
        selected_assembly: &HashSet<usize>,
        pc: Option<u8>,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut i = 0;
        let mut no_space = false;
        let selected_colour = visuals
            .strong_text_color()
            .lerp_to_gamma(Color32::CYAN.lerp_to_gamma(Color32::BLUE, 0.4), 0.5);
        for CompiledLine {
            page_start,
            page_end,
            assembly_line_num,
            ..
        } in lines
        {
            if page_start == page_end {
                //zero-sized assembly command e.g. meta commands like .LABEL
                if selected_assembly.contains(assembly_line_num) {
                    job.append(
                        "|",
                        0.0,
                        TextFormat {
                            color: selected_colour,
                            ..Default::default()
                        },
                    );
                    no_space = true;
                }
            } else {
                if i != 0 && !no_space {
                    job.append(
                        " ",
                        0.0,
                        TextFormat {
                            ..Default::default()
                        },
                    );
                }
                i += 1;
                no_space = false;

                for i in page_start.map(|p| p as usize).unwrap_or(256)
                    ..page_end.map(|p| p as usize).unwrap_or(256)
                {
                    job.append(
                        &page[i..(i + 1)],
                        0.0,
                        TextFormat {
                            color: if selected_assembly.contains(assembly_line_num) {
                                selected_colour
                            } else if pc.is_some_and(|pc| pc as usize == i) {
                                visuals.strong_text_color()
                            } else {
                                visuals.text_color()
                            },
                            ..Default::default()
                        },
                    );
                }
            }
        }
        job
    }

    ui.add(
        egui::TextEdit::multiline(&mut nibbles)
            .font(egui::TextStyle::Monospace)
            .desired_rows(1)
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .interactive(false)
            .layouter(&mut layouter),
    );
}

fn page_raw(ui: &mut Ui, nibbles: Vec<Nibble>, pc: Option<u8>) {
    let mut nibbles = nibbles.iter().map(|n| n.hex_str()).collect::<String>();

    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job(text.as_str(), ui.visuals(), pc);
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

    fn layout_job(page: &str, visuals: &Visuals, pc: Option<u8>) -> LayoutJob {
        let mut job = LayoutJob::default();
        debug_assert_eq!(page.len(), 256);
        for i in 0..page.len() {
            debug_assert!(i < 256);
            job.append(
                &page[i..(i + 1)],
                0.0,
                TextFormat {
                    color: if pc.is_some_and(|pc| i == pc as usize) {
                        visuals.strong_text_color()
                    } else {
                        visuals.text_color()
                    },
                    ..Default::default()
                },
            );
        }
        job
    }

    ui.add(
        egui::TextEdit::multiline(&mut nibbles)
            .font(egui::TextStyle::Monospace)
            .desired_rows(1)
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .interactive(false)
            .layouter(&mut layouter),
    );
}

fn make_torch_rom_page(blocks: &mut Blocks, ox: i32, oy: i32, oz: i32, nibbles: Vec<Nibble>) {
    assert_eq!(nibbles.len(), 256);
    fn set_nibble(schem: &mut Blocks, x: i32, y: i32, z: i32, n: Nibble) {
        for i in 0usize..4 {
            let dx = -2 * i as i32;
            let block = if n.as_usize() & (1 << (3 - i)) != 0 {
                Block::Plain {
                    id: "minecraft:redstone_wall_torch[facing=north]".into(),
                }
            } else {
                Block::Plain {
                    id: "minecraft:glass".into(),
                }
            };
            schem.place((x + dx, y, z), &block);
        }
    }

    for (i, n) in nibbles.iter().enumerate() {
        let (q, r) = (i / 32, i % 32);
        set_nibble(blocks, ox - 8 * q as i32, oy, oz - 2 * r as i32, *n);
    }
}

fn make_barrel_rom_page(blocks: &mut Blocks, ox: i32, oy: i32, oz: i32, nibbles: Vec<Nibble>) {
    assert_eq!(nibbles.len(), 256);
    for a in 0usize..8 {
        for d in 0usize..32 {
            let pos = (ox - 2 * d as i32, oy - 2 * a as i32, oz);
            let ss = nibbles[d + 32 * a];
            if ss == Nibble::N0 {
                blocks.place(
                    pos,
                    &Block::Plain {
                        id: "minecraft:glass".into(),
                    },
                );
            } else {
                blocks.place(pos, &Block::Barrel { ss });
            }
        }
    }
}

fn place_rom_page(blocks: &mut Blocks, page: Nibble, memory: &assembly::ProgramPage) {
    let page = page.as_usize();
    match page {
        0 => {
            println!("Schematics for ROM page 0 are not supported.");
        }
        1..=3 => {
            make_torch_rom_page(
                blocks,
                -5,
                -10 - 5 * (page as i32 - 1),
                -5,
                memory.nibbles(),
            );
        }
        4..=15 => {
            make_barrel_rom_page(
                blocks,
                -13,
                -11 - if page.is_multiple_of(2) { 16 } else { 0 },
                13 + 4 * ((page as i32 - 4) / 2),
                memory.nibbles(),
            );
        }
        _ => {
            panic!("Invalid ROM page {}", page);
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
        offset: (i32, i32, i32),
        first: Option<&Block>,
        aligned: Option<&Block>,
        between: Option<&Block>,
        join: Option<&Block>,
    ) {
        if let Some(first) = first {
            schem.place(self.coords.local_to_global_pos(offset), first);
        }
        if let Some(aligned) = aligned {
            let mut dz = 2i32;
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
            let mut dz = 3i32;
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
            let mut dz = 1i32;
            for &size in &self.section_sizes {
                schem.place(
                    self.coords
                        .local_to_global_pos((offset.0, offset.1, offset.2 + dz)),
                    join,
                );
                dz += 2 * size as i32;
            }
        }
    }

    fn place_data(&mut self, schem: &mut Blocks, data: Vec<Vec<bool>>) {
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
            let mut dz = 2i32;
            for (i, &size) in self.section_sizes.iter().enumerate() {
                #[allow(clippy::needless_range_loop)]
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

// input is a list of (addr, value) pairs to write
fn place_ram_data(blocks: &mut Blocks, values: Vec<(u16, u16)>) {
    println!("{:?}", values);

    let mut state = RamCard {
        coords: Coords {
            transform: Transform::translate((47, -49, -78)),
        },
        section_sizes: vec![8, 6, 8, 8],
        data_block: Block::Plain {
            id: "minecraft:gray_wool".into(),
        },
        read_block: Block::Plain {
            id: "minecraft:lime_wool".into(),
        },
    };

    state.place_start(blocks);

    let mut i = 0;
    let layer_at_i = 8;
    for (addr, value) in values {
        // Data
        {
            if i == layer_at_i {
                i = 0;
                state.place_new_layer(blocks);
            }
            state.place_data(
                blocks,
                vec![
                    (0..8).map(|i| (addr >> i) & 1 != 0).collect(),
                    (8..12)
                        .map(|i| (addr >> i) & 1 != 0)
                        .chain(vec![false, true])
                        .collect(),
                    (0..8).map(|i| (value >> i) & 1 != 0).collect(),
                    (8..16).map(|i| (value >> i) & 1 != 0).collect(),
                ],
            );
            i += 1;
        }
        {
            // Dummy for more delay
            if i == layer_at_i {
                i = 0;
                state.place_new_layer(blocks);
            }
            state.place_data(
                blocks,
                vec![
                    (0..8).map(|_| false).collect(),
                    (8..12).map(|_| false).chain(vec![false, false]).collect(),
                    (0..8).map(|_| false).collect(),
                    (8..16).map(|_| false).collect(),
                ],
            );
            i += 1;
        }
    }
}

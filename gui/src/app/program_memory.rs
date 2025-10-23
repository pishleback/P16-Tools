use crate::app::simulator::SimulatorStateTrait;
use crate::app::state::State;
use assembly::ProgramPagePtr;
use assembly::{CompiledLine, FullCompileResult, Nibble};
use egui::{Color32, TextBuffer, TextFormat, Ui, Visuals, text::LayoutJob};
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
mod save_schem {
    use schemgen::Schem;
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    /// Trigger a browser download of arbitrary binary data
    #[wasm_bindgen]
    fn download_binary_file(filename: &str, bytes: &[u8]) {
        // Convert the Rust &[u8] slice into a JavaScript Uint8Array
        let uint8_array = Uint8Array::from(bytes);

        // Put it into a JS array, as Blob::new_with_u8_array_sequence expects an array of parts
        let parts = js_sys::Array::new();
        parts.push(&uint8_array);

        // Create a binary blob
        let blob = Blob::new_with_u8_array_sequence_and_options(
            &parts,
            BlobPropertyBag::new().type_("application/octet-stream"), // generic binary MIME type
        )
        .unwrap();

        // Create a temporary object URL for the blob
        let url = Url::create_object_url_with_blob(&blob).unwrap();

        // Create an <a> element and click it to trigger download
        let document = web_sys::window().unwrap().document().unwrap();
        let a = document
            .create_element("a")
            .unwrap()
            .dyn_into::<HtmlAnchorElement>()
            .unwrap();

        a.set_href(&url);
        a.set_download(filename);
        a.click();

        // Clean up
        Url::revoke_object_url(&url).unwrap();
    }

    pub fn save(schem: Schem) {
        let mut bytes: Vec<u8> = vec![];
        schem.finish(&mut bytes);
        download_binary_file("p16_program.schem", &bytes);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod save_schem {
    use schemgen::Schem;

    pub fn save(schem: Schem) {
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

                if ui.button("Save Schematic").clicked() {
                    let mut schem = schemgen::Schem::new();
                    for i in 1u8..16 {
                        let i = Nibble::new(i).unwrap();
                        schem.place_rom_page(i, raw_memory.rom_page(i));
                    }
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

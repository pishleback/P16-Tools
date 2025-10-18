use crate::app::state::State;
use assembly::ProgramPagePtr;
use assembly::{CompiledLine, FullCompileResult, Nibble};
use egui::{Color32, TextBuffer, TextFormat, Ui, Visuals, text::LayoutJob};
use std::collections::HashSet;

pub fn update(
    state: &State,
    compile_result: &FullCompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    if let Ok((Ok((Ok(compiled), _page_layout)), _assembly)) = &compile_result {
        let raw_memory = compiled.memory().clone();

        #[cfg(not(target_arch = "wasm32"))]
        let simulator = state.simulator.simulator();
        #[cfg(target_arch = "wasm32")]
        let simulator: Option<&super::simulator::SimulatorState> = None;

        egui::CollapsingHeader::new("Memory").show(ui, |ui| {
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
                                simulator
                                    .map(|s| s.get_pc())
                                    .and_then(|ptr| match ptr.page {
                                        ProgramPagePtr::Rom { page } => {
                                            if page == rom_page {
                                                Some(ptr.counter)
                                            } else {
                                                None
                                            }
                                        }
                                        ProgramPagePtr::Ram { .. } => None,
                                    }),
                            );
                        },
                    );
                }
            }

            // Show RAM pages
            for (ram_page_num, ram_page) in compiled.ram_pages().into_iter().enumerate() {
                let live_nibbles = simulator
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
                                    simulator
                                        .map(|s| s.get_pc())
                                        .and_then(|ptr| match ptr.page {
                                            ProgramPagePtr::Rom { .. } => None,
                                            ProgramPagePtr::Ram { addr } => {
                                                if addr == ram_page.start {
                                                    Some(ptr.counter)
                                                } else {
                                                    None
                                                }
                                            }
                                        }),
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
                                simulator
                                    .map(|s| s.get_pc())
                                    .and_then(|ptr| match ptr.page {
                                        ProgramPagePtr::Rom { .. } => None,
                                        ProgramPagePtr::Ram { addr } => {
                                            if addr == ram_page.start {
                                                Some(ptr.counter)
                                            } else {
                                                None
                                            }
                                        }
                                    }),
                            );
                        });
                }
            }

            #[cfg(false)]
            {
                let ram_data = simulator.map_or(raw_memory.ram().data().to_vec(), |s| {
                    s.get_memory().ram().data().to_vec()
                });

                let max_chars = {
                    let available_width = ui.available_width();
                    let char_width = ui.fonts(|fonts| {
                        fonts.glyph_width(&egui::TextStyle::Monospace.resolve(ui.style()), '0')
                    });
                    // theoretical answer
                    let max_chars = (available_width / char_width).floor() as usize;
                    // but it seems to be off a bit
                    let max_chars = max_chars.saturating_sub(3);
                    std::cmp::max(max_chars, 1)
                };

                let pad_to_len = |mut s: String, n: usize| -> String {
                    while s.len() < n {
                        s += " ";
                    }
                    s
                };

                let str_data = ram_data
                    .into_iter()
                    .map(|v| format!("{v}"))
                    .collect::<Vec<_>>();
                let entry_len = std::cmp::max(str_data.iter().map(|s| s.len()).max().unwrap(), 4);
                let str_data = str_data
                    .into_iter()
                    .map(|s| pad_to_len(s, entry_len))
                    .collect::<Vec<_>>();

                let entries_per_row = {
                    // biggest possible
                    let entries_per_row =
                        std::cmp::max(max_chars.saturating_sub(4) / (entry_len + 1), 1);
                    // // but lets find the largest possible power of 2
                    // let mut i = 0usize;
                    // while (1 << (i + 1)) < entries_per_row {
                    //     i += 1;
                    // }
                    // let entries_per_row = 1 << i;
                    entries_per_row
                };

                let rows = vec![
                    vec![String::from("    ")]
                        .into_iter()
                        .chain(
                            (0..entries_per_row)
                                .map(|i| pad_to_len(String::from("FFFF"), entry_len)),
                        )
                        .collect::<Vec<_>>(),
                ]
                .into_iter()
                .chain(str_data.chunks(entries_per_row).map(|row| {
                    vec![String::from("ABCD")]
                        .into_iter()
                        .chain(row.to_vec())
                        .collect::<Vec<_>>()
                }))
                .collect::<Vec<_>>();

                let mut z = rows
                    .into_iter()
                    .map(|row| row.join(" "))
                    .collect::<Vec<_>>()
                    .join("\n");

                egui::CollapsingHeader::new("RAM").show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .stick_to_bottom(false)
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut z)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(1)
                                    .lock_focus(true)
                                    .interactive(false)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });
            }
        });
    }
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
            job.append(
                &page[*page_start..*page_end],
                0.0,
                TextFormat {
                    color: if selected_assembly.contains(assembly_line_num) {
                        selected_colour
                    } else if pc.is_some_and(|pc| {
                        (*page_start <= pc as usize) && ((pc as usize) < *page_end)
                    }) {
                        visuals.strong_text_color()
                    } else {
                        visuals.text_color()
                    },
                    ..Default::default()
                },
            );
        }
    }
    job
}

fn page_raw(ui: &mut Ui, nibbles: Vec<Nibble>, pc: Option<u8>) {
    let mut nibbles = nibbles.iter().map(|n| n.hex_str()).collect::<String>();

    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job_raw(text.as_str(), ui.visuals(), pc);
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

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

fn layout_job_raw(page: &str, visuals: &Visuals, pc: Option<u8>) -> LayoutJob {
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

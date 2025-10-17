use std::collections::HashSet;

use crate::app::state::State;
use assembly::{CompiledLine, FullCompileResult, Nibble, RAM_SIZE};
use egui::{Color32, TextBuffer, TextFormat, Ui, Visuals, text::LayoutJob};

pub fn update(
    state: &State,
    compile_result: &FullCompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    if let Ok((Ok((Ok(compiled), page_layout)), _assembly)) = &compile_result {
        let raw_memory = compiled.memory().clone();

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
                            );
                        },
                    );
                }
            }

            // Show RAM pages
            for (ram_page_num, ram_page) in compiled.ram_pages().into_iter().enumerate() {
                let nibbles = raw_memory.ram_page(ram_page.start).nibbles();
                let lines = compiled.ram_lines(ram_page_num);
                if !lines.is_empty() {
                    egui::CollapsingHeader::new(format!("RAM {}", ram_page_num)).show(ui, |ui| {
                        page(
                            ui,
                            nibbles,
                            lines,
                            state.selected_lines.as_ref().unwrap_or(&HashSet::new()),
                        );
                    });
                }
            }

            #[cfg(false)]
            {
                // Show RAM as u16 numbers
                let mut ram_data = (0..RAM_SIZE as usize)
                    .map(|i| raw_memory.ram().data()[i])
                    .collect::<Vec<_>>();
                while ram_data.last().map(|v| *v == 0).unwrap_or(false) {
                    ram_data.pop();
                }
                if !ram_data.is_empty() {
                    let mut z = ram_data
                        .into_iter()
                        .map(|v| format!("{v}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    egui::CollapsingHeader::new("RAM").show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut z)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(1)
                                .lock_focus(true)
                                .desired_width(f32::INFINITY),
                        );
                    });
                }
            }
        });
    }
}

fn page(
    ui: &mut Ui,
    nibbles: Vec<Nibble>,
    lines: &Vec<CompiledLine>,
    selected_assembly: &HashSet<usize>,
) {
    let mut nibbles = nibbles.iter().map(|n| n.hex_str()).collect::<String>();

    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job(text.as_str(), ui.visuals(), lines, selected_assembly);
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

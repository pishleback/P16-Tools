use crate::app::simulator::SimulatorStateTrait;
use crate::app::state::State;
use assembly::RamMem;
use assembly::{FullCompileResult, Nibble};
use egui::{TextBuffer, TextFormat, Ui, Visuals, text::LayoutJob};

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
enum RamDataFormat {
    #[default]
    Hex,
    Dec,
    Bin,
}

#[derive(Default)]
pub struct MemoryState {
    ram_data_format: RamDataFormat,
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

                ui.horizontal(|ui| {
                    ui.label("RAM Formatting");
                    egui::ComboBox::from_id_salt("RAM Value Format")
                        .selected_text(match &state.memory.ram_data_format {
                            RamDataFormat::Hex => "Hex",
                            RamDataFormat::Dec => "Decimal",
                            RamDataFormat::Bin => "Binary",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.memory.ram_data_format,
                                RamDataFormat::Hex,
                                "Hex",
                            );
                            ui.selectable_value(
                                &mut state.memory.ram_data_format,
                                RamDataFormat::Dec,
                                "Decimal",
                            );
                            ui.selectable_value(
                                &mut state.memory.ram_data_format,
                                RamDataFormat::Bin,
                                "Binary",
                            );
                        });
                });

                let ram_data = state
                    .simulator
                    .simulator()
                    .map_or(raw_memory.ram().clone(), |s| s.get_memory().ram().clone());
                ram(ui, ram_data, state.memory.ram_data_format);
            }
        });
}

fn ram(ui: &mut Ui, ram: RamMem, ram_data_format: RamDataFormat) {
    let mut layouter = |ui: &egui::Ui, _text: &dyn TextBuffer, wrap_width: f32| {
        let max_chars = {
            let char_width = ui.fonts(|fonts| {
                fonts.glyph_width(&egui::TextStyle::Monospace.resolve(ui.style()), '0')
            });
            // theoretical answer
            let max_chars = (wrap_width / char_width).floor() as usize;
            // // but it seems to be off a bit
            let max_chars = (975 * max_chars) / 1000;
            std::cmp::max(max_chars, 1)
        };

        let mut job = layout_job(ui.visuals(), max_chars, &ram, ram_data_format);
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

    fn layout_job(
        visuals: &Visuals,
        max_width: usize,
        ram: &RamMem,
        ram_data_format: RamDataFormat,
    ) -> LayoutJob {
        let rpad_to_len = |mut s: String, n: usize, c: char| -> String {
            while s.len() < n {
                s += &String::from(c);
            }
            s
        };
        let lpad_to_len = |mut s: String, n: usize, c: char| -> String {
            while s.len() < n {
                s = String::from(c) + &s;
            }
            s
        };

        let format_dec = (
            |v: u16| -> String { rpad_to_len(format!("{}", v), 5, ' ') },
            5usize,
        );
        let format_hex = (
            |v: u16| -> String {
                format!(
                    "{}{}{}{}",
                    Nibble::new(((v >> 12) & 15u16) as u8).unwrap().hex_str(),
                    Nibble::new(((v >> 8) & 15u16) as u8).unwrap().hex_str(),
                    Nibble::new(((v >> 4) & 15u16) as u8).unwrap().hex_str(),
                    Nibble::new((v & 15u16) as u8).unwrap().hex_str()
                )
            },
            4usize,
        );
        let format_bin = (
            |v: u16| -> String { lpad_to_len(format!("{:b}", v), 16, '0') },
            16usize,
        );

        let (format_value, value_width): (Box<dyn Fn(u16) -> String>, usize) = match ram_data_format
        {
            RamDataFormat::Bin => (Box::new(|x| format_bin.0(x)), format_bin.1),
            RamDataFormat::Hex => (Box::new(|x| format_hex.0(x)), format_hex.1),
            RamDataFormat::Dec => (Box::new(|x| format_dec.0(x)), format_dec.1),
        };
        let (format_addr, addr_width) = format_hex;

        let col_width = std::cmp::max(value_width, addr_width);

        let cols_power_of_2 = {
            let mut power_of_2 = 0usize;
            loop {
                let width = addr_width + (col_width + 1) * (1usize << (power_of_2 + 1));
                if width > max_width {
                    break;
                }
                power_of_2 += 1;
            }
            power_of_2
        };
        let cols = 1usize << cols_power_of_2;

        let mut job: LayoutJob = LayoutJob::default();

        // Top row
        job.append(
            &String::from(" ").repeat(addr_width),
            0.0,
            TextFormat {
                font_id: egui::FontId::monospace(12.0),
                ..Default::default()
            },
        );
        for i in 0..cols {
            job.append(
                " ",
                0.0,
                TextFormat {
                    font_id: egui::FontId::monospace(12.0),
                    ..Default::default()
                },
            );
            job.append(
                &rpad_to_len(format_addr(i as u16), col_width, ' '),
                0.0,
                TextFormat {
                    font_id: egui::FontId::monospace(12.0),
                    color: visuals.strong_text_color(),
                    ..Default::default()
                },
            );
        }

        // Other rows
        for (i, values) in ram.data().chunks(cols).enumerate() {
            job.append(
                "\n",
                0.0,
                TextFormat {
                    font_id: egui::FontId::monospace(12.0),
                    ..Default::default()
                },
            );
            job.append(
                &format_addr((i * cols) as u16),
                0.0,
                TextFormat {
                    font_id: egui::FontId::monospace(12.0),
                    color: visuals.strong_text_color(),
                    ..Default::default()
                },
            );
            for value in values {
                job.append(
                    " ",
                    0.0,
                    TextFormat {
                        font_id: egui::FontId::monospace(12.0),
                        ..Default::default()
                    },
                );
                job.append(
                    &rpad_to_len(format_value(*value), col_width, ' '),
                    0.0,
                    TextFormat {
                        font_id: egui::FontId::monospace(12.0),
                        ..Default::default()
                    },
                );
            }
        }
        job
    }

    let mut s = String::from(""); // Unused
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .stick_to_bottom(false)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut s)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(1)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .interactive(false)
                    .layouter(&mut layouter),
            );
        });
}

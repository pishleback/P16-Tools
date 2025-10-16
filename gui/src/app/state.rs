use assembly::{FullCompileResult, Nibble, RAM_SIZE, full_compile};
use egui::{Color32, RichText};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct State {
    pub text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            text: "PASS".into(),
        }
    }
}

impl State {
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let source = self.text.clone();
        let compile_result: FullCompileResult = full_compile(&source);

        let compiled_memory = compile_result
            .clone()
            .ok()
            .and_then(|inner| inner.0.ok().and_then(|inner| inner.0.ok()))
            .map(|compile_success| compile_success.program_memory);

        // Left panel with buttons
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Test Buttons");
                if ui.button("Clear").clicked() {
                    self.text.clear();
                }

                if compiled_memory.is_none() {
                    ui.heading("Compile Error");

                    match &compile_result {
                        Ok((result, _)) => match result {
                            Ok((result, _)) => match result {
                                Ok(_) => {}
                                Err(e) => match e {
                                    assembly::CompileError::Invalid16BitValue { .. } => {
                                        ui.label(
                                            RichText::new("Invalid 16-bit immediate value.")
                                                .color(Color32::RED),
                                        );
                                    }
                                    assembly::CompileError::MissingLabel { label, .. } => {
                                        ui.label(
                                            RichText::new(format!(
                                                "Label `{}` not defined.",
                                                label.t.to_string()
                                            ))
                                            .color(Color32::RED),
                                        );
                                    }
                                    assembly::CompileError::JumpOrBranchToOtherPage { .. } => {
                                        ui.label(
                                            RichText::new(
                                                "\
JUMP or BRANCH to a different page is not possible. Use CALL to chage pages.",
                                            )
                                            .color(Color32::RED),
                                        );
                                    }
                                    assembly::CompileError::BadUseflagsWithBranch { .. } => {
                                        ui.label(
                                            RichText::new("BadUseflagsWithBranch")
                                                .color(Color32::RED),
                                        );
                                    }
                                    assembly::CompileError::BadUseflags { .. } => {
                                        ui.label(RichText::new("BadUseflags").color(Color32::RED));
                                    }
                                    assembly::CompileError::PageFull { page } => match page {
                                        assembly::PageIdent::Rom(nibble) => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "ROM page {} is full.",
                                                    nibble.hex_str()
                                                ))
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::PageIdent::Ram(_) => {
                                            ui.label(
                                                RichText::new("RAM is full.").color(Color32::RED),
                                            );
                                        }
                                    },
                                },
                            },
                            Err(e) => match e {
                                assembly::LayoutPagesError::DuplicateLabel { label, .. } => {
                                    ui.label(
                                        RichText::new(format!(
                                            "Duplicate Label Definition: `{label}`"
                                        ))
                                        .color(Color32::RED),
                                    );
                                }
                                assembly::LayoutPagesError::MissingPageStart { .. } => {
                                    ui.label(
                                        RichText::new(
                                            "\
Missing page definition. Add `..ROM <page>` or `..RAM` before instructions."
                                                .to_string(),
                                        )
                                        .color(Color32::RED),
                                    );
                                }
                            },
                        },
                        Err(e) => match e {
                            lalrpop_util::ParseError::InvalidToken { .. } => {
                                ui.label(RichText::new("Invalid Token").color(Color32::RED));
                            }
                            lalrpop_util::ParseError::UnrecognizedEof { expected, .. } => {
                                ui.label(
                                    RichText::new(format!(
                                        "Unrecognized EOF. Expected one of: {}",
                                        expected.join(", ")
                                    ))
                                    .color(Color32::RED),
                                );
                            }
                            lalrpop_util::ParseError::UnrecognizedToken { expected, .. } => {
                                ui.label(
                                    RichText::new(format!(
                                        "Unrecognized Token. Expected one of: {}",
                                        expected.join(", ")
                                    ))
                                    .color(Color32::RED),
                                );
                            }
                            lalrpop_util::ParseError::ExtraToken { .. } => {
                                ui.label(RichText::new("Extra Token").color(Color32::RED));
                            }
                            lalrpop_util::ParseError::User { .. } => {
                                ui.label(RichText::new("Parse Error").color(Color32::RED));
                            }
                        },
                    }
                }
            });

        // Central text area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Assembly").show(ui, |ui| {
                        super::assembly_text_box::update(self, &compile_result, ctx, frame, ui);
                    });

                    if let Some(compiled_memory) = compiled_memory {
                        egui::CollapsingHeader::new("Memory").show(ui, |ui| {
                            // Show ROM pages
                            for n in (0..16).map(|n| Nibble::new(n).unwrap()) {
                                let mut z = compiled_memory
                                    .rom_page(n)
                                    .nibbles()
                                    .iter()
                                    .map(|n| n.hex_str())
                                    .collect::<String>();
                                z = String::from(z.trim_end_matches('0'));
                                if !z.is_empty() {
                                    egui::CollapsingHeader::new(format!("ROM {}", n.hex_str()))
                                        .show(ui, |ui| {
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
                            // Show RAM
                            let mut ram_data = (0..RAM_SIZE as usize)
                                .map(|i| compiled_memory.ram().data()[i])
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
                        });
                    }

                    egui::CollapsingHeader::new("Simulator").show(ui, |ui| {
                        ui.columns(3, |ui| {
                            ui[0].label("Column 111111");
                            ui[1].label("Column 2");
                            ui[2].label("Column 3");
                        });

                        ui.label("TODO: Simulator");
                    });
                });
        });
    }
}

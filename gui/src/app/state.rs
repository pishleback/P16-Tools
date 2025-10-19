#[cfg(not(target_arch = "wasm32"))]
use crate::app::simulator;
use assembly::{FullCompileResult, full_compile};
use egui::{Color32, RichText};
use std::collections::HashSet;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct State {
    pub source: String,
    #[serde(skip)]
    pub selected_lines: Option<HashSet<usize>>, // which lines of assembly are highlighted
    #[serde(skip)]
    #[cfg(not(target_arch = "wasm32"))]
    pub simulator: simulator::State<simulator::multithreaded::SimulatorState>,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            source: "".into(),
            selected_lines: None,
            #[cfg(not(target_arch = "wasm32"))]
            simulator: simulator::State::default(),
        };
        #[cfg(not(target_arch = "wasm32"))]
        state.simulator.update_source(&state.source);
        state
    }
}

impl State {
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let source = self.source.clone();
        let compile_result: FullCompileResult = full_compile(&source);

        let compiled_memory = compile_result
            .clone()
            .ok()
            .and_then(|inner| inner.0.ok().and_then(|inner| inner.0.ok()))
            .map(|compile_success| compile_success.memory().clone());

        #[cfg(not(target_arch = "wasm32"))]
        self.simulator.update_source(&source);

        egui::Window::new("Assembly").show(ctx, |ui| {
            super::assembly::update(self, &compile_result, ctx, frame, ui);
        });

        egui::Window::new("Memory").show(ctx, |ui| {
            super::memory::update(self, &compile_result, ctx, frame, ui);
        });

        #[cfg(not(target_arch = "wasm32"))]
        egui::Window::new("Simulator").show(ctx, |ui| {
            simulator::update(&mut self.simulator, ctx, frame, ui);
        });

        // Central text area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(false)
                .show(ui, |ui| {
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
                                                    "Page location label `{}` not defined.",
                                                    label.t.to_string()
                                                ))
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::MissingConstLabel {
                                            label, ..
                                        } => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Const label `{}` not defined.",
                                                    label.t.to_string()
                                                ))
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::DuplicateConstLabel {
                                            label,
                                            ..
                                        } => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Duplicate Const label definition: `{}`",
                                                    label.t.to_string()
                                                ))
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::JumpOrBranchToOtherPage {
                                            ..
                                        } => {
                                            ui.label(
                                                RichText::new(
                                                    "\
JUMP or BRANCH to a different page is not possible. Use CALL to chage pages.",
                                                )
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::BadUseflagsWithBranch {
                                            ..
                                        } => {
                                            ui.label(
                                                RichText::new(
                                                    "\
BRANCH does not use flags at .USEFLAGS and it is not \
possible to fix with extra PASS instructions.",
                                                )
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::BadUseflags { .. } => {
                                            ui.label(
                                                RichText::new("BadUseflags").color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::RomPageFull { page } => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "ROM page {} is full.",
                                                    page.hex_str()
                                                ))
                                                .color(Color32::RED),
                                            );
                                        }
                                        assembly::CompileError::RamFull => {
                                            ui.label(
                                                RichText::new("RAM is full.").color(Color32::RED),
                                            );
                                        }

                                        assembly::CompileError::InvalidCommandLocation {
                                            ..
                                        } => {
                                            ui.label(
                                                RichText::new(
                                                    "Line appears in an invalid location."
                                                        .to_string(),
                                                )
                                                .color(Color32::RED),
                                            );
                                        }
                                    },
                                },
                                Err(e) => match e {
                                    assembly::LayoutPagesError::DuplicateLabel {
                                        label, ..
                                    } => {
                                        ui.label(
                                            RichText::new(format!(
                                                "Duplicate label: `{}`",
                                                label.t.to_string()
                                            ))
                                            .color(Color32::RED),
                                        );
                                    }
                                    assembly::LayoutPagesError::Invalid16BitConstValue {
                                        ..
                                    } => {
                                        ui.label(
                                            RichText::new("Invalid 16-bit constant value.")
                                                .color(Color32::RED),
                                        );
                                    }
                                    assembly::LayoutPagesError::DuplicateConstLabel {
                                        label,
                                        ..
                                    } => {
                                        ui.label(
                                            RichText::new(format!(
                                                "Duplicate label: `{}`",
                                                label.t.to_string()
                                            ))
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
                                lalrpop_util::ParseError::UnrecognizedToken {
                                    expected, ..
                                } => {
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
        });
    }
}

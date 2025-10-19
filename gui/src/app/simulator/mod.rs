use assembly::{EndErrorState, Nibble, ProgramMemory, ProgramPtr, Simulator, full_compile};
use egui::{RichText, Slider};

#[cfg(not(target_arch = "wasm32"))]
pub mod multithreaded;
#[cfg(target_arch = "wasm32")]
pub mod singlethreaded;

#[derive(Debug, Clone, Copy)]
pub enum SimulatorEndState {
    Halt,
    Killed,
    Error(EndErrorState),
}

pub trait SimulatorStateTrait {
    // Maximum x such that 10^x is the max instructions per second on the slider
    const MAX_ISP_EXP: f64;

    fn new(simulator: Simulator, instructions_per_second: f64) -> Self;
    fn set_instructions_per_second(&mut self, instructions_per_second: f64);
    fn get_instructions_per_second(&mut self) -> f64;
    fn end_state(&mut self) -> Option<SimulatorEndState>;
    fn one_step(&mut self);

    fn process(&mut self, max_time: std::time::Duration);

    fn get_reg(&self, nibble: Nibble) -> u16;
    fn get_pc(&self) -> ProgramPtr;
    fn get_memory(&self) -> ProgramMemory;
    fn get_data_stack(&mut self) -> Vec<u16>;
}

pub struct State<SimulatorState: SimulatorStateTrait> {
    source: String,
    sim_speed_slider: f64,
    simulator: Option<SimulatorState>,
}

impl<SimulatorState: SimulatorStateTrait> Default for State<SimulatorState> {
    fn default() -> Self {
        Self {
            source: Default::default(),
            sim_speed_slider: Default::default(),
            simulator: Default::default(),
        }
    }
}

impl<SimulatorState: SimulatorStateTrait> State<SimulatorState> {
    pub fn is_compiled(&self) -> bool {
        self.simulator.is_some()
    }

    pub fn update_source(&mut self, new_source: &String) {
        if &self.source != new_source {
            self.source = new_source.clone();
            self.reload_simulator();
        }
    }

    pub fn reload_simulator(&mut self) {
        let memory = full_compile(&self.source)
            .ok()
            .and_then(|inner| inner.0.ok().and_then(|inner| inner.0.ok()))
            .map(|compile_success| compile_success.memory().clone());
        self.simulator = memory.map(|m| {
            SimulatorState::new(
                m.simulator(),
                Self::instructions_per_second_from_sim_speed_slider(self.sim_speed_slider),
            )
        });
    }

    fn instructions_per_second_from_sim_speed_slider(t: f64) -> f64 {
        if t <= 0.0 {
            0.0
        } else {
            10f64.powf(SimulatorState::MAX_ISP_EXP * t)
        }
    }

    pub fn simulator(&self) -> Option<&SimulatorState> {
        self.simulator.as_ref()
    }
}

pub fn update<SimulatorState: SimulatorStateTrait>(
    state: &mut State<SimulatorState>,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    if let Some(simulator) = state.simulator.as_mut() {
        simulator.process(std::time::Duration::from_millis(10));
    }

    if state.is_compiled() {
        ui.horizontal(|ui| {
            if let Some(simulator) = state.simulator.as_mut() {
                match simulator.end_state() {
                    Some(end) => match end {
                        SimulatorEndState::Halt => {
                            ui.label("Finished");
                        }
                        SimulatorEndState::Killed => {
                            ui.label("Killed");
                        }
                        SimulatorEndState::Error(e) => match e {
                            EndErrorState::DataStackOverflow => {
                                ui.label("Data Stack Overflow");
                            }
                        },
                    },
                    None => {
                        if simulator.get_instructions_per_second() == 0.0 {
                            ui.label("Paused");
                            if ui.button("Step").clicked() {
                                simulator.one_step();
                            }
                        } else {
                            ui.label("Running");
                            ctx.request_repaint();
                        }
                    }
                }
            }

            if ui.button("Reset").clicked() {
                state.reload_simulator();
            }
        });

        ui.horizontal(|ui| {
            if let Some(simulator) = state.simulator.as_mut() {
                ui.label("Instructions Per Second");

                ui.add(
                    Slider::new(&mut state.sim_speed_slider, 0.0..=1.0).custom_formatter(|t, _| {
                        let ips =
                            State::<SimulatorState>::instructions_per_second_from_sim_speed_slider(
                                t,
                            );
                        if ips <= 0.0 {
                            "0".to_string()
                        } else if ips < 10.0 {
                            format!("{:.1}", ips)
                        } else {
                            format!("{:.0}", ips)
                        }
                    }),
                );

                simulator.set_instructions_per_second(
                    State::<SimulatorState>::instructions_per_second_from_sim_speed_slider(
                        state.sim_speed_slider,
                    ),
                );
            }
        });

        if let Some(simulator) = state.simulator.as_mut() {
            egui::CollapsingHeader::new("Program Counter").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let pc = simulator.get_pc();
                    match pc.page {
                        assembly::ProgramPagePtr::Rom { page } => {
                            ui.label(format!("Page = ROM {}", page.hex_str()));
                        }
                        assembly::ProgramPagePtr::Ram { addr } => {
                            ui.label(format!("Page = RAM {}", addr));
                        }
                    }
                    ui.label(format!("Counter = {}", pc.counter));
                })
            });

            egui::CollapsingHeader::new("Registers").show(ui, |ui| {
                for reg in 0..16 {
                    let reg = Nibble::new(reg).unwrap();
                    show_16bit_value(ui, format!("%{}", reg.hex_str()), simulator.get_reg(reg));
                }
            });

            egui::CollapsingHeader::new("Data Stack").show(ui, |ui| {
                let data_stack = simulator.get_data_stack();
                for n in data_stack {
                    show_16bit_value(ui, String::new(), n);
                }
            });
        }
    }
}

fn show_16bit_value(ui: &mut egui::Ui, label: String, value: u16) {
    let box_size = egui::vec2(16.0, 16.0);

    // temporarily override spacing inside this scope
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0); // very small horizontal spacing

    // draw the bits
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).text_style(egui::TextStyle::Monospace));
        for i in (0..16).rev() {
            // MSB on the left
            let bit_on = (value >> i) & 1 == 1;
            let color = if bit_on {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().code_bg_color
            };

            let (rect, _response) = ui.allocate_exact_size(box_size, egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::BLACK),
                egui::StrokeKind::Middle,
            );
        }
    });

    ui.spacing_mut().item_spacing = old_spacing;
}

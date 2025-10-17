use assembly::{EndErrorState, Nibble, ProgramPtr, Simulator, full_compile};
use egui::{RichText, Slider};
use std::{
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};

#[derive(Debug, Clone, Copy)]
enum SimulatorEndState {
    Halt,
    Killed,
    Error(EndErrorState),
}

struct SimulatorState {
    simulator: Arc<Mutex<Simulator>>,
    instructions_per_second: Arc<Mutex<f64>>,
    instructions_to_do: Arc<Mutex<f64>>,

    stop: Arc<Mutex<bool>>,
    run_thread: Option<JoinHandle<SimulatorEndState>>,
    run_result: Option<SimulatorEndState>,

    largest_data_stack: usize,
}

impl Drop for SimulatorState {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
    }
}

impl SimulatorState {
    fn new(simulator: Simulator, instructions_per_second: f64) -> Self {
        let simulator = Arc::new(Mutex::new(simulator));
        let stop = Arc::new(Mutex::new(false));
        let instructions_per_second = Arc::new(Mutex::new(instructions_per_second));
        let instructions_to_do = Arc::new(Mutex::new(0.0));
        Self {
            simulator: simulator.clone(),
            instructions_per_second: instructions_per_second.clone(),
            instructions_to_do: instructions_to_do.clone(),
            stop: stop.clone(),
            run_thread: Some(spawn(move || {
                let mut prev_time = std::time::SystemTime::now();

                while !*stop.lock().unwrap() {
                    // calculate how many instructions to run to keep in line with desired instructions per second
                    let now_time = std::time::SystemTime::now();
                    let dt = now_time.duration_since(prev_time).unwrap().as_secs_f64();
                    prev_time = now_time;
                    let n = {
                        let instructions_per_second = instructions_per_second.lock().unwrap();
                        let mut instructions_to_do = instructions_to_do.lock().unwrap();
                        *instructions_to_do += dt * *instructions_per_second;
                        let n = instructions_to_do.floor() as usize;
                        *instructions_to_do -= n as f64;
                        *instructions_to_do = instructions_to_do.clamp(0.0, 1.0);
                        n
                    };

                    for i in 0..n {
                        // check we've not been in the loop too long to keep us responsive to instructions_per_second
                        if i % 65536 == 0
                            && std::time::SystemTime::now()
                                .duration_since(prev_time)
                                .unwrap()
                                .as_secs_f64()
                                > 1.0
                        {
                            break;
                        }

                        let step_result = simulator.lock().unwrap().step(false);
                        match step_result {
                            Ok(state) => match state {
                                assembly::EndStepOkState::Continue => {}
                                assembly::EndStepOkState::WaitingForInput => {
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                                assembly::EndStepOkState::Finish => {
                                    return SimulatorEndState::Halt;
                                }
                            },
                            Err(e) => {
                                return SimulatorEndState::Error(e);
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                SimulatorEndState::Killed
            })),
            run_result: None,
            largest_data_stack: 0,
        }
    }

    fn end_state(&mut self) -> Option<SimulatorEndState> {
        if self.run_thread.is_some() {
            let run_thread = self.run_thread.take().unwrap();
            if run_thread.is_finished() {
                self.run_result = Some(run_thread.join().unwrap());
            } else {
                self.run_thread = Some(run_thread);
            }
        }
        self.run_result
    }

    fn one_step(&mut self) {
        *self.instructions_to_do.lock().unwrap() += 1.0;
    }

    fn set_instructions_per_second(&mut self, instructions_per_second: f64) {
        *self.instructions_per_second.lock().unwrap() = instructions_per_second;
    }

    fn get_reg(&self, nibble: Nibble) -> u16 {
        self.simulator.lock().unwrap().get_reg(nibble)
    }

    fn get_pc(&self) -> ProgramPtr {
        self.simulator.lock().unwrap().get_pc()
    }

    fn get_data_stack(&mut self) -> Vec<u16> {
        let mut data_stack = self
            .simulator
            .lock()
            .unwrap()
            .get_data_stack()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        self.largest_data_stack = std::cmp::max(data_stack.len(), self.largest_data_stack);
        while data_stack.len() < self.largest_data_stack {
            data_stack.push(0);
        }
        data_stack
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct State {
    #[serde(skip)]
    source: String,
    sim_speed_slider: f64,
    #[serde(skip)]
    simulator: Option<SimulatorState>,
}

impl State {
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
        if t <= 0.0 { 0.0 } else { 10f64.powf(9.0 * t) }
    }
}

pub fn update(
    state: &mut State,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
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
                    if *simulator.instructions_per_second.lock().unwrap() == 0.0 {
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
                    let ips = State::instructions_per_second_from_sim_speed_slider(t);
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
                State::instructions_per_second_from_sim_speed_slider(state.sim_speed_slider),
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

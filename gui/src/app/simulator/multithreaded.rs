use crate::app::simulator::{SimulatorEndState, SimulatorStateTrait};
use assembly::{Nibble, ProgramMemory, ProgramPtr, Simulator};
use std::{
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};

pub struct SimulatorState {
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

impl SimulatorStateTrait for SimulatorState {
    const MAX_ISP_EXP: f64 = 9.0;

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

    fn set_instructions_per_second(&mut self, instructions_per_second: f64) {
        *self.instructions_per_second.lock().unwrap() = instructions_per_second;
    }

    fn get_instructions_per_second(&mut self) -> f64 {
        *self.instructions_per_second.lock().unwrap()
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

    fn get_reg(&self, nibble: Nibble) -> u16 {
        self.simulator.lock().unwrap().get_reg(nibble)
    }

    fn get_pc(&self) -> ProgramPtr {
        self.simulator.lock().unwrap().get_pc()
    }

    fn get_memory(&self) -> ProgramMemory {
        self.simulator.lock().unwrap().get_memory()
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

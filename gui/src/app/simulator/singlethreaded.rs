use crate::app::simulator::{SimulatorEndState, SimulatorStateTrait};
use assembly::{Nibble, ProgramMemory, ProgramPtr, Simulator};

#[allow(dead_code)]
pub struct SimulatorState {
    simulator: Simulator,
    instructions_per_second: f64,
    prev_time: chrono::DateTime<chrono::Utc>,
    end_state: Option<SimulatorEndState>,
    instructions_to_do: f64,
}

impl SimulatorStateTrait for SimulatorState {
    const MAX_ISP_EXP: f64 = 7.0;

    fn new(simulator: Simulator, instructions_per_second: f64) -> Self {
        Self {
            simulator,
            instructions_per_second,
            prev_time: chrono::Utc::now(),
            instructions_to_do: 0.0,
            end_state: None,
        }
    }

    fn set_instructions_per_second(&mut self, instructions_per_second: f64) {
        self.instructions_per_second = instructions_per_second;
    }

    fn get_instructions_per_second(&mut self) -> f64 {
        self.instructions_per_second
    }

    fn end_state(&mut self) -> Option<SimulatorEndState> {
        self.end_state
    }

    fn one_step(&mut self) {
        if self.end_state.is_none() {
            match self.simulator.step(false) {
                Ok(s) => match s {
                    assembly::EndStepOkState::Continue => {}
                    assembly::EndStepOkState::WaitingForInput => {}
                    assembly::EndStepOkState::Finish => {
                        self.end_state = Some(SimulatorEndState::Halt);
                    }
                },
                Err(e) => {
                    self.end_state = Some(SimulatorEndState::Error(e));
                }
            }
        }
    }

    fn process(&mut self, max_time: chrono::TimeDelta) {
        let start_time = chrono::Utc::now();
        self.instructions_to_do +=
            self.instructions_per_second * (start_time - self.prev_time).as_seconds_f64();
        while chrono::Utc::now() - start_time < max_time && self.instructions_to_do > 1.0 {
            self.one_step();
            self.instructions_to_do -= 1.0;
        }
        println!("{:?}", self.instructions_to_do);
        self.instructions_to_do = self.instructions_to_do.clamp(0.0, 1.0);
        self.prev_time = start_time;
    }

    fn get_reg(&self, nibble: Nibble) -> u16 {
        self.simulator.get_reg(nibble)
    }

    fn get_pc(&self) -> ProgramPtr {
        self.simulator.get_pc()
    }

    fn get_memory(&self) -> ProgramMemory {
        self.simulator.get_memory()
    }

    fn get_data_stack(&mut self) -> Vec<u16> {
        self.simulator
            .get_data_stack()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    }
}

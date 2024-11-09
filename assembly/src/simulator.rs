use std::{collections::VecDeque, thread::sleep, time::Duration};

use crate::memory::{Memory, Nibble, RomPage};

#[derive(Debug, Clone, Copy)]
pub enum EndErrorState {
    DataStackOverflow,
}

impl Memory {
    pub fn run(&self) -> Result<(), EndErrorState> {
        Simulator::new(self.clone()).run()
    }

    fn read(&self, ptr: &ProgramPtr) -> Nibble {
        match ptr.page {
            ProgramPagePtr::Rom { page } => self.rom_page(page).get_nibble(ptr.counter),
            ProgramPagePtr::Ram { addr } => {
                let nibble_block = self
                    .ram()
                    .get_value(addr.wrapping_add((ptr.counter / 4) as u16));
                let nibble_idx = ptr.counter % 4;
                Nibble::new(((nibble_block >> (nibble_idx * 4)) & 15u16) as u8)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ProgramPagePtr {
    Rom { page: Nibble },
    Ram { addr: u16 },
}

#[derive(Debug, Clone, Copy)]
struct ProgramPtr {
    page: ProgramPagePtr,
    counter: u8,
}
impl ProgramPtr {
    fn increment(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }
}

#[derive(Debug, Clone, Copy)]
struct AluFlags {
    zero: bool,
    negative: bool,
    carry: bool,
    overflow: bool,
}
fn add_with_flags(a: u16, b: u16) -> (u16, AluFlags) {
    let s = a.wrapping_add(b);
    (s, {
        let s_flags = noop_get_flags(s);
        let carry_last = (a as u32 + b as u32) & (1u32 << 16) != 0;
        let carry_second_to_last =
            ((a & !(1 << 15)) as u32 + (b & !(1 << 15)) as u32) & (1u32 << 15) != 0;
        AluFlags {
            zero: s_flags.zero,
            negative: s_flags.negative,
            carry: carry_last,
            overflow: carry_last ^ carry_second_to_last,
        }
    })
}
fn noop_get_flags(a: u16) -> AluFlags {
    AluFlags {
        zero: a == 0,
        negative: a >= (1 << 15),
        carry: false,
        overflow: false,
    }
}

struct Simulator {
    memory: Memory,
    program_counter: ProgramPtr,
    call_stack: Vec<ProgramPtr>,
    data_stack: Vec<u16>,
    registers: [u16; 16],
    flags_delay: VecDeque<AluFlags>,
    flags: AluFlags,
}
impl Simulator {
    fn new(memory: Memory) -> Self {
        Self {
            memory,
            program_counter: ProgramPtr {
                page: ProgramPagePtr::Rom {
                    page: Nibble::new(0),
                },
                counter: 0,
            },
            call_stack: vec![],
            data_stack: vec![],
            registers: [0; 16],
            flags_delay: vec![
                AluFlags {
                    zero: true,
                    negative: false,
                    carry: false,
                    overflow: false,
                };
                6
            ]
            .into(),
            flags: AluFlags {
                zero: true,
                negative: false,
                carry: false,
                overflow: false,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EndStepOkState {
    Continue,
    Finish,
}

impl Simulator {
    fn set_flags(&mut self, flags: AluFlags, past: usize) {
        let n = self.flags_delay.len();
        self.flags = flags;
        for i in 0..past {
            self.flags_delay[n - i - 1] = flags;
        }
    }

    fn increment(&mut self) {
        self.program_counter.increment();
        self.flags_delay.push_back(self.flags);
        self.flags_delay.pop_front();
    }

    fn push_data_stack(&mut self, x: u16) -> Result<(), EndErrorState> {
        self.data_stack.push(x);
        // return EndErrorState::DataStackOverflow;
        Ok(())
    }

    fn pop_data_stack(&mut self) -> u16 {
        self.data_stack.pop().unwrap_or(0)
    }

    fn get_reg_mut(&mut self, reg: Nibble) -> &mut u16 {
        &mut self.registers[reg.as_usize()]
    }

    fn step(&mut self) -> Result<EndStepOkState, EndErrorState> {
        let opcode = self.memory.read(&self.program_counter);
        match opcode.as_enum() {
            crate::Nibble::N0 => {
                println!("Pass");
                self.increment();
            }
            crate::Nibble::N1 => {
                println!("Value");
                self.increment();
                let n3 = self.memory.read(&self.program_counter);
                self.increment();
                let n2 = self.memory.read(&self.program_counter);
                self.increment();
                let n1 = self.memory.read(&self.program_counter);
                self.increment();
                let n0 = self.memory.read(&self.program_counter);
                self.increment();
                let value = n0.as_u16()
                    | n1.as_u16().wrapping_shl(4)
                    | n2.as_u16().wrapping_shl(8)
                    | n3.as_u16().wrapping_shl(12);
                self.push_data_stack(value)?;
            }
            crate::Nibble::N2 => {
                println!("Jump");
                self.increment();
                let a1 = self.memory.read(&self.program_counter);
                self.increment();
                let a0 = self.memory.read(&self.program_counter);
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                self.program_counter.counter = addr;
            }
            crate::Nibble::N3 => {
                println!("Branch");
                let flags_now = *self.flags_delay.front().unwrap();
                self.increment();
                let cond = self.memory.read(&self.program_counter);
                self.increment();
                let a1 = self.memory.read(&self.program_counter);
                self.increment();
                let a0 = self.memory.read(&self.program_counter);
                self.increment();
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                if match cond.as_enum() {
                    crate::Nibble::N0 => todo!(),
                    crate::Nibble::N1 => todo!(),
                    crate::Nibble::N2 => flags_now.zero,
                    crate::Nibble::N3 => todo!(),
                    crate::Nibble::N4 => todo!(),
                    crate::Nibble::N5 => todo!(),
                    crate::Nibble::N6 => todo!(),
                    crate::Nibble::N7 => todo!(),
                    crate::Nibble::N8 => todo!(),
                    crate::Nibble::N9 => todo!(),
                    crate::Nibble::N10 => todo!(),
                    crate::Nibble::N11 => todo!(),
                    crate::Nibble::N12 => todo!(),
                    crate::Nibble::N13 => todo!(),
                    crate::Nibble::N14 => todo!(),
                    crate::Nibble::N15 => todo!(),
                } {
                    self.program_counter.counter = addr;
                }
            }
            crate::Nibble::N4 => {
                println!("Push");
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let value = *self.get_reg_mut(reg);
                self.push_data_stack(value)?;
            }
            crate::Nibble::N5 => {
                println!("Pop");
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                *self.get_reg_mut(reg) = self.pop_data_stack();
            }
            crate::Nibble::N6 => {
                println!("Call");
                self.increment();
                let a1 = self.memory.read(&self.program_counter);
                self.increment();
                let a0 = self.memory.read(&self.program_counter);
                self.increment();
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                self.call_stack.push(self.program_counter);
                self.program_counter = ProgramPtr {
                    page: self.program_counter.page.clone(),
                    counter: addr,
                };
            }
            crate::Nibble::N7 => {
                println!("Return");
                match self.call_stack.pop() {
                    Some(ptr) => {
                        self.program_counter = ptr;
                    }
                    None => {
                        return Ok(EndStepOkState::Finish);
                    }
                }
            }
            crate::Nibble::N8 => {
                println!("Add");
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let acc_value = self.pop_data_stack();
                let reg_value = *self.get_reg_mut(reg);
                let (s, flags) = add_with_flags(acc_value, reg_value);
                self.push_data_stack(s)?;
                self.flags = flags;
            }
            crate::Nibble::N9 => {
                println!("Rotate");
                self.increment();
                let shift = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.get_reg_mut(reg);
                *reg = reg.rotate_left(shift.as_u32());
            }
            crate::Nibble::N10 => {
                print!("Alm1: ");
                self.increment();
                let op = self.memory.read(&self.program_counter);
                self.increment();
                match op.as_enum() {
                    crate::Nibble::N0 => todo!(),
                    crate::Nibble::N1 => {
                        println!("Not");
                        let x = self.pop_data_stack();
                        let y = !x;
                        self.set_flags(noop_get_flags(y), 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N2 => todo!(),
                    crate::Nibble::N3 => todo!(),
                    crate::Nibble::N4 => todo!(),
                    crate::Nibble::N5 => todo!(),
                    crate::Nibble::N6 => todo!(),
                    crate::Nibble::N7 => todo!(),
                    crate::Nibble::N8 => todo!(),
                    crate::Nibble::N9 => todo!(),
                    crate::Nibble::N10 => todo!(),
                    crate::Nibble::N11 => todo!(),
                    crate::Nibble::N12 => todo!(),
                    crate::Nibble::N13 => todo!(),
                    crate::Nibble::N14 => todo!(),
                    crate::Nibble::N15 => todo!(),
                }
            }
            crate::Nibble::N11 => {
                print!("Alm2: ");
                self.increment();
                let op = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                match op.as_enum() {
                    crate::Nibble::N0 => todo!(),
                    crate::Nibble::N1 => todo!(),
                    crate::Nibble::N2 => todo!(),
                    crate::Nibble::N3 => todo!(),
                    crate::Nibble::N4 => todo!(),
                    crate::Nibble::N5 => todo!(),
                    crate::Nibble::N6 => todo!(),
                    crate::Nibble::N7 => todo!(),
                    crate::Nibble::N8 => todo!(),
                    crate::Nibble::N9 => todo!(),
                    crate::Nibble::N10 => todo!(),
                    crate::Nibble::N11 => todo!(),
                    crate::Nibble::N12 => todo!(),
                    crate::Nibble::N13 => todo!(),
                    crate::Nibble::N14 => todo!(),
                    crate::Nibble::N15 => todo!(),
                }
            }
            crate::Nibble::N12 => {
                println!("RomCall");
                self.increment();
                let page = self.memory.read(&self.program_counter);
                self.increment();
                let b = self.memory.read(&self.program_counter);
                self.increment();
                let a = self.memory.read(&self.program_counter);
                self.increment();
                self.call_stack.push(self.program_counter);
                self.program_counter = ProgramPtr {
                    page: ProgramPagePtr::Rom { page: page },
                    counter: a.as_u8() | (b.as_u8() << 4),
                };
            }
            crate::Nibble::N13 => {
                println!("RamCall");
                self.increment();
                let b = self.memory.read(&self.program_counter);
                self.increment();
                let a = self.memory.read(&self.program_counter);
                self.increment();
                self.call_stack.push(self.program_counter);
                let addr = self.pop_data_stack();
                self.program_counter = ProgramPtr {
                    page: ProgramPagePtr::Ram { addr: addr },
                    counter: a.as_u8() | (b.as_u8() << 4),
                };
            }
            crate::Nibble::N14 => {
                println!("Input");
                todo!();
            }
            crate::Nibble::N15 => {
                println!("Output");
                todo!();
            }
        }
        Ok(EndStepOkState::Continue)
    }

    fn run(&mut self) -> Result<(), EndErrorState> {
        println!("Running");

        loop {
            let result = self.step()?;
            println!(
                "{:?} {:?} {:?}",
                self.program_counter, self.registers, self.data_stack
            );
            sleep(Duration::from_millis(100));
            match result {
                EndStepOkState::Continue => {}
                EndStepOkState::Finish => {
                    break;
                }
            }
        }

        Ok(())
    }
}

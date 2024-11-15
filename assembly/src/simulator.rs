use std::{
    collections::VecDeque,
    default,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use crate::{
    memory::{Memory, Nibble},
    OctDigit,
};

#[derive(Debug, Clone, Copy)]
pub enum EndErrorState {
    DataStackOverflow,
}

impl Memory {
    pub fn simulator(self) -> Simulator {
        Simulator::new(self)
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
fn add_with_flags(a: u16, b: u16, cin: bool) -> (u16, AluFlags) {
    let c = match cin {
        false => 0,
        true => 1,
    };
    let s = a.wrapping_add(b.wrapping_add(c));
    (s, {
        let s_flags = noop_get_flags(s);
        let carry_last = (a as u32 + b as u32 + c as u32) & (1u32 << 16) != 0;
        let carry_second_to_last =
            ((a & !(1 << 15)) as u32 + (b & !(1 << 15)) as u32 + c as u32) & (1u32 << 15) != 0;
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

pub struct Simulator {
    memory: Memory,
    program_counter: ProgramPtr,
    call_stack: Vec<ProgramPtr>,
    data_stack: Vec<u16>,
    registers: [u16; 16],
    flags_delay: VecDeque<AluFlags>,
    flags: AluFlags,
    input_queue: Arc<Mutex<InputQueue>>,
    output_targets: Vec<Box<dyn FnMut(Vec<OctDigit>, u16) -> ()>>,
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
            input_queue: Arc::new(Mutex::new(InputQueue::new())),
            output_targets: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EndStepOkState {
    Continue,
    Finish,
}

#[derive(Debug)]
pub struct InputQueue {
    queue: VecDeque<u16>,
}
impl InputQueue {
    fn new() -> Self {
        Self { queue: [].into() }
    }
    pub fn push(&mut self, val: u16) {
        self.queue.push_back(val);
    }
    fn pop(&mut self) -> Option<u16> {
        self.queue.pop_front()
    }
}

impl Simulator {
    pub fn subscribe_to_output(&mut self, callback: Box<dyn FnMut(Vec<OctDigit>, u16) -> ()>) {
        self.output_targets.push(callback);
    }

    pub fn input(&mut self) -> Arc<Mutex<InputQueue>> {
        self.input_queue.clone()
    }

    fn set_flags(&mut self, flags: AluFlags, past: usize) {
        let n = self.flags_delay.len();
        self.flags = flags;
        for i in 0..past {
            self.flags_delay[n - i - 1] = flags;
        }
    }

    fn flush_flag_delay(&mut self) {
        for i in 0..self.flags_delay.len() {
            self.flags_delay[i] = self.flags;
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

    fn step(&mut self, log_instructions: bool) -> Result<EndStepOkState, EndErrorState> {
        let opcode = self.memory.read(&self.program_counter);
        match opcode.as_enum() {
            crate::Nibble::N0 => {
                if log_instructions {
                    println!("Pass");
                }
                self.increment();
            }
            crate::Nibble::N1 => {
                if log_instructions {
                    println!("Value");
                }
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
                if log_instructions {
                    println!("Jump");
                }
                self.increment();
                let a1 = self.memory.read(&self.program_counter);
                self.increment();
                let a0 = self.memory.read(&self.program_counter);
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                self.program_counter.counter = addr;
                self.flush_flag_delay();
            }
            crate::Nibble::N3 => {
                if log_instructions {
                    println!("Branch");
                }
                let f = *self.flags_delay.front().unwrap(); // The flags to be used by the branch condition
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
                    crate::Nibble::N2 => f.zero,
                    crate::Nibble::N3 => !f.zero,
                    crate::Nibble::N4 => f.negative,
                    crate::Nibble::N5 => !f.negative,
                    crate::Nibble::N6 => f.overflow,
                    crate::Nibble::N7 => !f.overflow,
                    crate::Nibble::N8 => f.carry,
                    crate::Nibble::N9 => !f.carry,
                    crate::Nibble::N10 => f.carry && !f.zero,
                    crate::Nibble::N11 => !f.carry || f.zero,
                    crate::Nibble::N12 => f.negative == f.overflow,
                    crate::Nibble::N13 => f.negative != f.overflow,
                    crate::Nibble::N14 => f.negative == f.overflow && !f.zero,
                    crate::Nibble::N15 => f.negative != f.overflow || f.zero,
                } {
                    self.program_counter.counter = addr;
                }
                self.flush_flag_delay(); //Branch pauses long enough whether or not the branch was taken
            }
            crate::Nibble::N4 => {
                if log_instructions {
                    println!("Push");
                }
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let value = *self.get_reg_mut(reg);
                self.push_data_stack(value)?;
            }
            crate::Nibble::N5 => {
                if log_instructions {
                    println!("Pop");
                }
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                *self.get_reg_mut(reg) = self.pop_data_stack();
            }
            crate::Nibble::N6 => {
                if log_instructions {
                    println!("Call");
                }
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
                self.flush_flag_delay();
            }
            crate::Nibble::N7 => {
                if log_instructions {
                    println!("Return");
                }
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
                if log_instructions {
                    println!("Add");
                }
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let acc_value = self.pop_data_stack();
                let reg_value = *self.get_reg_mut(reg);
                let (s, flags) = add_with_flags(acc_value, reg_value, false);
                self.push_data_stack(s)?;
                self.flags = flags;
            }
            crate::Nibble::N9 => {
                if log_instructions {
                    println!("Rotate");
                }
                self.increment();
                let shift = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.get_reg_mut(reg);
                *reg = reg.rotate_left(shift.as_u32());
            }
            crate::Nibble::N10 => {
                if log_instructions {
                    print!("Alm1: ");
                }
                self.increment();
                let op = self.memory.read(&self.program_counter);
                self.increment();
                match op.as_enum() {
                    crate::Nibble::N0 => {
                        if log_instructions {
                            println!("Duplicate");
                        }
                        let x = self.pop_data_stack();
                        self.push_data_stack(x).unwrap();
                        self.push_data_stack(x)?;
                    }
                    crate::Nibble::N1 => {
                        if log_instructions {
                            println!("Not");
                        }
                        let x = self.pop_data_stack();
                        let y = !x;
                        self.set_flags(noop_get_flags(y), 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N2 => todo!(),
                    crate::Nibble::N3 => todo!(),
                    crate::Nibble::N4 => {
                        if log_instructions {
                            println!("Increment");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, 0, true);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N5 => {
                        if log_instructions {
                            println!("Increment With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, 0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N6 => {
                        if log_instructions {
                            println!("Decrement");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !0, false);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N7 => {
                        if log_instructions {
                            println!("Decrement With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N8 => {
                        if log_instructions {
                            println!("Negate");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(!x, 0, true);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N9 => {
                        if log_instructions {
                            println!("Negate With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(!x, 0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N10 => {
                        if log_instructions {
                            println!("Set Flags Without Pop");
                        }
                        let x = self.pop_data_stack();
                        self.set_flags(noop_get_flags(x), 2);
                        self.push_data_stack(x).unwrap();
                    }
                    crate::Nibble::N11 => {
                        if log_instructions {
                            println!("Set Flags With Pop");
                        }
                        let x = self.pop_data_stack();
                        self.set_flags(noop_get_flags(x), 2);
                    }
                    crate::Nibble::N12 => {
                        if log_instructions {
                            println!("Right Shift");
                        }
                        let x = self.pop_data_stack();
                        let (y, c) = (x >> 1, x & 1 != 0);
                        let mut f = noop_get_flags(y);
                        f.carry = c;
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N13 => {
                        if log_instructions {
                            println!("Right Shift With Carry");
                        }
                        let x = self.pop_data_stack();
                        let cin = self.flags.carry;
                        let (y, c) = (
                            (x >> 1)
                                | match cin {
                                    false => 0,
                                    true => 1 << 15,
                                },
                            x & 1 != 0,
                        );
                        let mut f = noop_get_flags(y);
                        f.carry = c;
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N14 => {
                        if log_instructions {
                            println!("Right Shift Carry In");
                        }
                        let x = self.pop_data_stack();
                        let (y, c) = ((x >> 1) | (1 << 15), x & 1 != 0);
                        let mut f = noop_get_flags(y);
                        f.carry = c;
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N15 => {
                        if log_instructions {
                            println!("Arithmetic Right Shift");
                        }
                        let x = self.pop_data_stack();
                        let cin = x & (1 << 15) != 0;
                        let (y, c) = (
                            (x >> 1)
                                | match cin {
                                    false => 0,
                                    true => 1 << 15,
                                },
                            x & 1 != 0,
                        );
                        let mut f = noop_get_flags(y);
                        f.carry = c;
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                }
            }
            crate::Nibble::N11 => {
                if log_instructions {
                    print!("Alm2: ");
                }
                self.increment();
                let op = self.memory.read(&self.program_counter);
                self.increment();
                let reg = self.memory.read(&self.program_counter);
                self.increment();
                let r = *self.get_reg_mut(reg);
                match op.as_enum() {
                    crate::Nibble::N0 => {
                        if log_instructions {
                            println!("Swap");
                        }
                        let x = self.pop_data_stack();
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(r).unwrap();
                    }
                    crate::Nibble::N1 => {
                        if log_instructions {
                            println!("Subtract");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N2 => todo!(),
                    crate::Nibble::N3 => todo!(),
                    crate::Nibble::N4 => {
                        if log_instructions {
                            println!("And");
                        }
                        let x = self.pop_data_stack();
                        let y = x & r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N5 => {
                        if log_instructions {
                            println!("NAnd");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x & r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N6 => {
                        if log_instructions {
                            println!("Or");
                        }
                        let x = self.pop_data_stack();
                        let y = x | r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N7 => {
                        if log_instructions {
                            println!("NOr");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x | r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N8 => {
                        if log_instructions {
                            println!("Xor");
                        }
                        let x = self.pop_data_stack();
                        let y = x ^ r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N9 => {
                        if log_instructions {
                            println!("NXor");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x ^ r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N10 => {
                        if log_instructions {
                            println!("Set Flags");
                        }
                        let f = noop_get_flags(r);
                        self.set_flags(f, 3);
                    }
                    crate::Nibble::N11 => {
                        if log_instructions {
                            println!("Compare");
                        }
                        let x = self.pop_data_stack();
                        self.push_data_stack(x).unwrap();
                        let (_y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                    }
                    crate::Nibble::N12 => {
                        if log_instructions {
                            println!("Swap Add");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, r, false);
                        self.set_flags(f, 3);
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N13 => {
                        if log_instructions {
                            println!("Swap Sub");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N14 => {
                        if log_instructions {
                            println!("Add With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, r, self.flags.carry);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    crate::Nibble::N15 => {
                        if log_instructions {
                            println!("Subtract With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !r, self.flags.carry);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                }
            }
            crate::Nibble::N12 => {
                if log_instructions {
                    println!("RomCall");
                }
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
                self.flush_flag_delay();
            }
            crate::Nibble::N13 => {
                if log_instructions {
                    println!("RamCall");
                }
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
                self.flush_flag_delay();
            }
            crate::Nibble::N14 => {
                if log_instructions {
                    println!("Input");
                }
                self.increment();
                loop {
                    let val_opt = self.input_queue.lock().unwrap().pop();
                    match val_opt {
                        Some(val) => {
                            self.push_data_stack(val)?;
                            break;
                        }
                        None => {
                            sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
            crate::Nibble::N15 => {
                if log_instructions {
                    println!("Output");
                }
                self.increment();
                let mut octs = vec![];
                loop {
                    let a = self.memory.read(&self.program_counter).as_u8();
                    let oct = OctDigit::new(a & 7);
                    octs.push(oct);
                    self.increment();

                    if a & 8 != 0 {
                        break;
                    }
                }
                let v = self.pop_data_stack();
                for output_target in &mut self.output_targets {
                    output_target(octs.clone(), v);
                }
            }
        }
        Ok(EndStepOkState::Continue)
    }

    pub fn run(&mut self, log_instructions: bool, log_state: bool) -> Result<(), EndErrorState> {
        loop {
            let result = self.step(log_instructions)?;
            let mut flags = vec![];
            if self.flags.zero {
                flags.push("Z");
            }
            if self.flags.negative {
                flags.push("N");
            }
            if self.flags.overflow {
                flags.push("V");
            }
            if self.flags.carry {
                flags.push("C");
            }
            if log_state {
                println!(
                    "    {:?} {:?} {:?} {:?}",
                    self.program_counter,
                    flags,
                    self.registers.iter().map(|n| *n as i16).collect::<Vec<_>>(),
                    self.data_stack
                        .iter()
                        .map(|n| *n as i16)
                        .collect::<Vec<_>>()
                );
            }
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

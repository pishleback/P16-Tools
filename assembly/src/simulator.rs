use crate::datatypes::{Nibble, OctDigit};
use crate::memory::ProgramMemory;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

#[derive(Debug, Clone, Copy)]
pub enum EndErrorState {
    DataStackOverflow,
}

impl ProgramMemory {
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
                let nibble_idx = 3 - ptr.counter % 4;
                Nibble::new(((nibble_block >> (nibble_idx * 4)) & 15u16) as u8).unwrap()
            }
        }
    }

    fn read_page(&self, page: ProgramPagePtr) -> [Nibble; 256] {
        let mut data = [Nibble::N0; 256];
        let mut ptr = ProgramPtr { page, counter: 0 };
        #[allow(clippy::needless_range_loop)]
        for i in 0..256 {
            data[i] = self.read(&ptr);
            ptr.increment();
        }
        data
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

pub type OutputTarget = Box<dyn FnMut(Vec<OctDigit>, u16)>;

pub struct Simulator {
    memory: ProgramMemory,
    program_counter: ProgramPtr,
    pcache: [Nibble; 256],
    call_stack: Vec<ProgramPtr>,
    data_stack: Vec<u16>,
    registers: [u16; 16],
    flags_delay: VecDeque<AluFlags>,
    flags: AluFlags,
    input_queue: Arc<Mutex<InputQueue>>,
    output_targets: Vec<OutputTarget>,
}

impl Simulator {
    fn new(memory: ProgramMemory) -> Self {
        let mut s = Self {
            memory,
            program_counter: ProgramPtr {
                page: ProgramPagePtr::Rom { page: Nibble::N0 },
                counter: 0,
            },
            pcache: [Nibble::N0; 256],
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
        };
        s.load_pache();
        s
    }

    pub fn subscribe_to_output(&mut self, callback: OutputTarget) {
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

    fn load_pache(&mut self) {
        self.pcache = self.memory.read_page(self.program_counter.page)
    }

    fn read_pcache(&self) -> Nibble {
        self.pcache[self.program_counter.counter as usize]
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
        let opcode = self.read_pcache();
        match opcode {
            Nibble::N0 => {
                if log_instructions {
                    println!("Pass");
                }
                self.increment();
            }
            Nibble::N1 => {
                if log_instructions {
                    println!("Value");
                }
                self.increment();
                let n3 = self.read_pcache();
                self.increment();
                let n2 = self.read_pcache();
                self.increment();
                let n1 = self.read_pcache();
                self.increment();
                let n0 = self.read_pcache();
                self.increment();
                let value = n0.as_u16()
                    | n1.as_u16().wrapping_shl(4)
                    | n2.as_u16().wrapping_shl(8)
                    | n3.as_u16().wrapping_shl(12);
                self.push_data_stack(value)?;
            }
            Nibble::N2 => {
                if log_instructions {
                    println!("Jump");
                }
                self.increment();
                let a1 = self.read_pcache();
                self.increment();
                let a0 = self.read_pcache();
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                self.program_counter.counter = addr;
                self.flush_flag_delay();
            }
            Nibble::N3 => {
                if log_instructions {
                    println!("Branch");
                }
                let f = *self.flags_delay.front().unwrap(); // The flags to be used by the branch condition
                self.increment();
                let cond = self.read_pcache();
                self.increment();
                let a1 = self.read_pcache();
                self.increment();
                let a0 = self.read_pcache();
                self.increment();
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                if match cond {
                    Nibble::N0 => !self.input_queue.lock().unwrap().queue.is_empty(),
                    Nibble::N1 => self.input_queue.lock().unwrap().queue.is_empty(),
                    Nibble::N2 => f.zero,
                    Nibble::N3 => !f.zero,
                    Nibble::N4 => f.negative,
                    Nibble::N5 => !f.negative,
                    Nibble::N6 => f.overflow,
                    Nibble::N7 => !f.overflow,
                    Nibble::N8 => f.carry,
                    Nibble::N9 => !f.carry,
                    Nibble::N10 => f.carry && !f.zero,
                    Nibble::N11 => !f.carry || f.zero,
                    Nibble::N12 => f.negative == f.overflow,
                    Nibble::N13 => f.negative != f.overflow,
                    Nibble::N14 => f.negative == f.overflow && !f.zero,
                    Nibble::N15 => f.negative != f.overflow || f.zero,
                } {
                    self.program_counter.counter = addr;
                }
                self.flush_flag_delay(); //Branch pauses long enough whether or not the branch was taken
            }
            Nibble::N4 => {
                if log_instructions {
                    println!("Push");
                }
                self.increment();
                let reg = self.read_pcache();
                self.increment();
                let value = *self.get_reg_mut(reg);
                self.push_data_stack(value)?;
            }
            Nibble::N5 => {
                if log_instructions {
                    println!("Pop");
                }
                self.increment();
                let reg = self.read_pcache();
                self.increment();
                *self.get_reg_mut(reg) = self.pop_data_stack();
            }
            Nibble::N6 => {
                if log_instructions {
                    println!("Call");
                }
                self.increment();
                let a1 = self.read_pcache();
                self.increment();
                let a0 = self.read_pcache();
                self.increment();
                let addr = a0.as_u8() | a1.as_u8().wrapping_shl(4);
                self.call_stack.push(self.program_counter);
                self.program_counter = ProgramPtr {
                    page: self.program_counter.page,
                    counter: addr,
                };
                self.load_pache();
                self.flush_flag_delay();
            }
            Nibble::N7 => {
                if log_instructions {
                    println!("Return");
                }
                match self.call_stack.pop() {
                    Some(ptr) => {
                        self.program_counter = ptr;
                        self.load_pache();
                    }
                    None => {
                        return Ok(EndStepOkState::Finish);
                    }
                }
            }
            Nibble::N8 => {
                if log_instructions {
                    println!("Add");
                }
                self.increment();
                let reg = self.read_pcache();
                self.increment();
                let acc_value = self.pop_data_stack();
                let reg_value = *self.get_reg_mut(reg);
                let (s, flags) = add_with_flags(acc_value, reg_value, false);
                self.push_data_stack(s)?;
                self.flags = flags;
            }
            Nibble::N9 => {
                if log_instructions {
                    println!("Rotate");
                }
                self.increment();
                let shift = self.read_pcache();
                self.increment();
                let reg = self.read_pcache();
                self.increment();
                let reg = self.get_reg_mut(reg);
                *reg = reg.rotate_left(shift.as_u32());
            }
            Nibble::N10 => {
                if log_instructions {
                    print!("Alm1: ");
                }
                self.increment();
                let op = self.read_pcache();
                self.increment();
                match op {
                    Nibble::N0 => {
                        if log_instructions {
                            println!("Duplicate");
                        }
                        let x = self.pop_data_stack();
                        self.push_data_stack(x).unwrap();
                        self.push_data_stack(x)?;
                    }
                    Nibble::N1 => {
                        if log_instructions {
                            println!("Not");
                        }
                        let x = self.pop_data_stack();
                        let y = !x;
                        self.set_flags(noop_get_flags(y), 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N2 => {
                        if log_instructions {
                            println!("Read");
                        }
                        let x = self.pop_data_stack();
                        self.input()
                            .lock()
                            .unwrap()
                            .push(self.memory.ram_mut().get_value(x));
                        self.push_data_stack(x).unwrap();
                    }
                    Nibble::N3 => {
                        if log_instructions {
                            println!("Read and Pop");
                        }
                        let x = self.pop_data_stack();
                        self.input()
                            .lock()
                            .unwrap()
                            .push(self.memory.ram_mut().get_value(x));
                    }
                    Nibble::N4 => {
                        if log_instructions {
                            println!("Increment");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, 0, true);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N5 => {
                        if log_instructions {
                            println!("Increment With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, 0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N6 => {
                        if log_instructions {
                            println!("Decrement");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !0, false);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N7 => {
                        if log_instructions {
                            println!("Decrement With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N8 => {
                        if log_instructions {
                            println!("Negate");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(!x, 0, true);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N9 => {
                        if log_instructions {
                            println!("Negate With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(!x, 0, self.flags.carry);
                        self.set_flags(f, 2);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N10 => {
                        if log_instructions {
                            println!("Set Flags Without Pop");
                        }
                        let x = self.pop_data_stack();
                        self.set_flags(noop_get_flags(x), 2);
                        self.push_data_stack(x).unwrap();
                    }
                    Nibble::N11 => {
                        if log_instructions {
                            println!("Set Flags With Pop");
                        }
                        let x = self.pop_data_stack();
                        self.set_flags(noop_get_flags(x), 2);
                    }
                    Nibble::N12 => {
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
                    Nibble::N13 => {
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
                    Nibble::N14 => {
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
                    Nibble::N15 => {
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
            Nibble::N11 => {
                if log_instructions {
                    print!("Alm2: ");
                }
                self.increment();
                let op = self.read_pcache();
                self.increment();
                let reg = self.read_pcache();
                self.increment();
                let r = *self.get_reg_mut(reg);
                match op {
                    Nibble::N0 => {
                        if log_instructions {
                            println!("Swap");
                        }
                        let x = self.pop_data_stack();
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(r).unwrap();
                    }
                    Nibble::N1 => {
                        if log_instructions {
                            println!("Subtract");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N2 => {
                        if log_instructions {
                            println!("Write");
                        }
                        let x = self.pop_data_stack();
                        self.memory.ram_mut().set_value(x, r);
                        self.push_data_stack(x).unwrap();
                    }
                    Nibble::N3 => {
                        if log_instructions {
                            println!("Write and Pop");
                        }
                        let x = self.pop_data_stack();
                        self.memory.ram_mut().set_value(x, r);
                    }
                    Nibble::N4 => {
                        if log_instructions {
                            println!("And");
                        }
                        let x = self.pop_data_stack();
                        let y = x & r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N5 => {
                        if log_instructions {
                            println!("NAnd");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x & r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N6 => {
                        if log_instructions {
                            println!("Or");
                        }
                        let x = self.pop_data_stack();
                        let y = x | r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N7 => {
                        if log_instructions {
                            println!("NOr");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x | r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N8 => {
                        if log_instructions {
                            println!("Xor");
                        }
                        let x = self.pop_data_stack();
                        let y = x ^ r;
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N9 => {
                        if log_instructions {
                            println!("NXor");
                        }
                        let x = self.pop_data_stack();
                        let y = !(x ^ r);
                        let f = noop_get_flags(y);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N10 => {
                        if log_instructions {
                            println!("Set Flags");
                        }
                        let f = noop_get_flags(r);
                        self.set_flags(f, 3);
                    }
                    Nibble::N11 => {
                        if log_instructions {
                            println!("Compare");
                        }
                        let x = self.pop_data_stack();
                        self.push_data_stack(x).unwrap();
                        let (_y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                    }
                    Nibble::N12 => {
                        if log_instructions {
                            println!("Swap Add");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, r, false);
                        self.set_flags(f, 3);
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N13 => {
                        if log_instructions {
                            println!("Swap Sub");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, !r, true);
                        self.set_flags(f, 3);
                        *self.get_reg_mut(reg) = x;
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N14 => {
                        if log_instructions {
                            println!("Add With Carry");
                        }
                        let x = self.pop_data_stack();
                        let (y, f) = add_with_flags(x, r, self.flags.carry);
                        self.set_flags(f, 3);
                        self.push_data_stack(y).unwrap();
                    }
                    Nibble::N15 => {
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
            Nibble::N12 => {
                if log_instructions {
                    println!("RomCall");
                }
                self.increment();
                let page = self.read_pcache();
                self.increment();
                let b = self.read_pcache();
                self.increment();
                let a = self.read_pcache();
                self.increment();
                self.call_stack.push(self.program_counter);
                self.program_counter = ProgramPtr {
                    page: ProgramPagePtr::Rom { page },
                    counter: a.as_u8() | (b.as_u8() << 4),
                };
                self.load_pache();
                self.flush_flag_delay();
            }
            Nibble::N13 => {
                if log_instructions {
                    println!("RamCall");
                }
                self.increment();
                let b = self.read_pcache();
                self.increment();
                let a = self.read_pcache();
                self.increment();
                self.call_stack.push(self.program_counter);
                let addr = self.pop_data_stack();
                self.program_counter = ProgramPtr {
                    page: ProgramPagePtr::Ram { addr },
                    counter: a.as_u8() | (b.as_u8() << 4),
                };
                self.load_pache();
                self.flush_flag_delay();
            }
            Nibble::N14 => {
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
            Nibble::N15 => {
                if log_instructions {
                    println!("Output");
                }
                self.increment();
                let mut octs = vec![];
                loop {
                    let a = self.read_pcache().as_u8();
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

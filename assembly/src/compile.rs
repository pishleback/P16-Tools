use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
};

use crate::assembly::{Assembly, Label, Line, Meta};
use crate::datatypes::Nibble;

#[derive(Debug)]
struct Memory {
    rom_pages: [[Option<Nibble>; 256]; 16],
    ram: [Option<Nibble>; 1 << (12 + 2)],
}
impl Memory {
    fn blank() -> Self {
        Self {
            rom_pages: [[None; 256]; 16],
            ram: [None; 1 << (12 + 2)],
        }
    }

    fn set_nibble(&mut self, ptr: MemNibblePtr, n: Nibble) -> Result<(), ()> {
        match ptr {
            MemNibblePtr::Rom(nibble, addr) => {
                let entry = &mut self.rom_pages[nibble.as_usize()][addr as usize];
                match entry {
                    Some(_) => Err(()),
                    None => {
                        *entry = Some(n);
                        Ok(())
                    }
                }
            }
            MemNibblePtr::Ram(addr) => {
                let entry = &mut self.ram[addr];
                match entry {
                    Some(_) => Err(()),
                    None => {
                        *entry = Some(n);
                        Ok(())
                    }
                }
            }
        }
    }

    fn finish(&self) -> super::memory::ProgramMemory {
        super::memory::ProgramMemory::new(
            core::array::from_fn(|i| {
                core::array::from_fn(|j| self.rom_pages[i][j].unwrap_or(Nibble::N0))
            }),
            core::array::from_fn(|i| self.ram[i].unwrap_or(Nibble::N0)),
        )
    }
}

#[derive(Debug, Clone)]
enum MemNibblePtr {
    Rom(Nibble, u8),
    Ram(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageLocation {
    Rom(Nibble),
    Ram(u16),
}
impl PageLocation {
    fn nibble_ptr(&self, a: u8) -> MemNibblePtr {
        match self {
            PageLocation::Rom(nibble) => MemNibblePtr::Rom(*nibble, a),
            PageLocation::Ram(base) => MemNibblePtr::Ram(*base as usize + a as usize),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PageIdent {
    Rom(Nibble),
    Ram(usize),
}

#[derive(Debug)]
struct MemoryManager {
    memory: Memory,
    rom_ptr: [u8; 16],
    ram_ptr: usize,
    label_values: HashMap<Label, (PageLocation, u8)>,
    label_targets: Vec<(Label, PageLocation, u8)>,
    ram_ident_to_addr: HashMap<usize, u16>,
    ram_addr_targets: Vec<(usize, PageLocation, u8)>,
}
impl MemoryManager {
    fn blank() -> Self {
        Self {
            memory: Memory::blank(),
            rom_ptr: [0; 16],
            ram_ptr: 0,
            label_values: HashMap::new(),
            label_targets: vec![],
            ram_ident_to_addr: HashMap::new(),
            ram_addr_targets: vec![],
        }
    }
    fn new_page<'a>(&'a mut self, page: PageIdent) -> MemoryPageManager<'a> {
        let (page, ptr) = match page {
            PageIdent::Rom(nibble) => (PageLocation::Rom(nibble), self.rom_ptr[nibble.as_usize()]),
            PageIdent::Ram(ident) => {
                while self.ram_ptr % 4 != 0 {
                    self.ram_ptr += 1;
                }
                if self.ram_ident_to_addr.contains_key(&ident) {
                    panic!();
                }
                let addr = (self.ram_ptr >> 2) as u16;
                self.ram_ident_to_addr.insert(ident, addr);
                (PageLocation::Ram(addr), 0)
            }
        };
        MemoryPageManager {
            memory_manager: self,
            page,
            ptr,
            flags_as_set: FlagsSetBy::Unknown,
            flags_delay_queue: (0..6).map(|_i| FlagsSetBy::Unknown).collect(),
        }
    }
    fn finish(mut self) -> Memory {
        // Replace labels with u8 page addresses
        for (label, blank_page, blank_ptr) in &self.label_targets {
            let (_target_page, target_ptr) = self.label_values.get(label).unwrap();
            let page = blank_page;
            self.memory
                .set_nibble(
                    page.nibble_ptr(*blank_ptr),
                    Nibble::new((target_ptr >> 4) & 15).unwrap(),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    page.nibble_ptr(*blank_ptr + 1),
                    Nibble::new(target_ptr & 15).unwrap(),
                )
                .unwrap();
        }
        // Replace tagged locations with ram addresses
        for (ident, blank_page, blank_ptr) in &self.ram_addr_targets {
            let addr = *self.ram_ident_to_addr.get(ident).unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 3),
                    Nibble::new((addr & 15) as u8).unwrap(),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 2),
                    Nibble::new(((addr >> 4) & 15) as u8).unwrap(),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 1),
                    Nibble::new(((addr >> 8) & 15) as u8).unwrap(),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr),
                    Nibble::new(((addr >> 12) & 15) as u8).unwrap(),
                )
                .unwrap();
        }
        self.memory
    }
}

#[derive(Debug, Clone, Copy)]
enum FlagsSetBy {
    Unreachable,
    Unknown,
    Nibble(u8),
}

#[derive(Debug)]
struct MemoryPageManager<'a> {
    memory_manager: &'a mut MemoryManager,
    page: PageLocation,
    ptr: u8,
    // Current start of the flags as
    flags_as_set: FlagsSetBy,
    flags_delay_queue: VecDeque<FlagsSetBy>,
}
impl<'a> MemoryPageManager<'a> {
    fn nibble_ptr(&self) -> MemNibblePtr {
        match self.page {
            PageLocation::Rom(nibble) => MemNibblePtr::Rom(nibble, self.ptr),
            PageLocation::Ram(base) => MemNibblePtr::Ram(4 * base as usize + self.ptr as usize),
        }
    }
    fn delayed_flags_for_branch(&self) -> FlagsSetBy {
        *self.flags_delay_queue.back().unwrap()
    }
    fn tick_flags_delay(&mut self) {
        self.flags_delay_queue.push_front(self.flags_as_set);
        self.flags_delay_queue.pop_back().unwrap();
    }
    fn flush_flags(&mut self) {
        self.flags_delay_queue = self
            .flags_delay_queue
            .iter()
            .map(|_f| self.flags_as_set)
            .collect();
    }
    fn set_flags(&mut self) {
        self.flags_as_set = FlagsSetBy::Nibble(self.ptr);
    }
    fn unknown_flags(&mut self) {
        self.flags_as_set = FlagsSetBy::Unknown;
        self.flush_flags();
    }
    fn unreachable_flags(&mut self) {
        self.flags_as_set = FlagsSetBy::Unreachable;
        self.flush_flags();
    }
    fn wait_for_flags(&mut self, flags_set_on: u8) -> Option<usize> {
        for (i, f) in self.flags_delay_queue.iter().rev().enumerate() {
            match f {
                FlagsSetBy::Unreachable => {}
                FlagsSetBy::Unknown => {}
                FlagsSetBy::Nibble(l) => {
                    if *l == flags_set_on {
                        return Some(i);
                    }
                }
            }
        }
        None
    }
    fn inc(&mut self) {
        self.ptr = match self.ptr.checked_add(1) {
            Some(ptr_plus_one) => ptr_plus_one,
            None => panic!("Page full"),
        };
        match self.page {
            PageLocation::Rom(nibble) => {
                self.memory_manager.rom_ptr[nibble.as_usize()] += 1;
            }
            PageLocation::Ram(_) => {
                self.memory_manager.ram_ptr += 1;
            }
        }
        self.tick_flags_delay();
    }
    fn push(&mut self, n: u8) {
        self.memory_manager
            .memory
            .set_nibble(self.nibble_ptr(), Nibble::new(n).unwrap())
            .unwrap();
        self.inc()
    }
    fn label_location(&mut self, label: Label) {
        if self.memory_manager.label_values.contains_key(&label) {
            panic!("Label already exists");
        }
        self.memory_manager
            .label_values
            .insert(label.clone(), (self.page, self.ptr));
    }
    fn label_target(&mut self, label: Label) {
        self.memory_manager
            .label_targets
            .push((label, self.page, self.ptr));
        self.inc();
        self.inc();
    }
    fn ram_addr(&mut self, ident: usize) {
        self.memory_manager
            .ram_addr_targets
            .push((ident, self.page, self.ptr));
        self.inc();
        self.inc();
        self.inc();
        self.inc();
    }
}

impl Assembly {
    pub fn compile(&self) -> super::memory::ProgramMemory {
        let mut pages: Vec<(PageIdent, Vec<Line>)> = vec![];
        let mut label_to_page: HashMap<Label, PageIdent> = HashMap::new();
        {
            let mut ram_page_ident_counter = 0;
            for line in self.lines() {
                if let crate::assembly::Line::Meta(Meta::Label(label)) = line {
                    if label_to_page.contains_key(label) {
                        panic!("Duplicate label `{}`", label.to_string());
                    }
                    label_to_page.insert(label.clone(), pages.last().unwrap().0);
                }

                match line {
                    crate::assembly::Line::Meta(Meta::RomPage(n)) => {
                        pages.push((PageIdent::Rom(*n), vec![]));
                    }
                    crate::assembly::Line::Meta(Meta::RamPage) => {
                        pages.push((PageIdent::Ram(ram_page_ident_counter), vec![]));
                        ram_page_ident_counter += 1;
                    }
                    _ => match pages.last_mut() {
                        Some((_, lines)) => {
                            lines.push(line.clone());
                        }
                        None => {
                            panic!("Probably forgot to specify the first page");
                        }
                    },
                }
            }
        }

        let mut code = MemoryManager::blank();
        {
            for (page, lines) in pages {
                let mut code = code.new_page(page);
                let mut useflags_line: Option<u8> = None;
                for line in lines {
                    match line {
                        Line::Command(command) => match command {
                            crate::assembly::Command::Pass => {
                                code.push(0);
                            }
                            crate::assembly::Command::Value(v) => {
                                code.push(1);
                                let a = (v & 15) as u8;
                                let b = ((v >> 4) & 15) as u8;
                                let c = ((v >> 8) & 15) as u8;
                                let d = ((v >> 12) & 15) as u8;
                                code.push(d);
                                code.push(c);
                                code.push(b);
                                code.push(a);
                            }
                            crate::assembly::Command::Jump(label) => {
                                if page != *label_to_page.get(&label).unwrap() {
                                    panic!("Cannot jump to a different page");
                                }
                                code.push(2);
                                code.label_target(label);
                            }
                            crate::assembly::Command::Branch(condition, label) => {
                                if page != *label_to_page.get(&label).unwrap() {
                                    panic!("Cannot branch to a different page");
                                }
                                match useflags_line {
                                    Some(useflags_line) => {
                                        match code.wait_for_flags(useflags_line) {
                                            Some(delay) => {
                                                for _ in 0..delay {
                                                    code.push(0);
                                                }
                                            }
                                            None => {
                                                match match code.delayed_flags_for_branch() {
                                                    FlagsSetBy::Unreachable => {
                                                        Err("Is unreachable".to_string())
                                                    }
                                                    FlagsSetBy::Unknown => Err("The flags could come from an unknown source".to_string()),
                                                    FlagsSetBy::Nibble(branch_line) => {
                                                        if useflags_line != branch_line {
                                                            Err(format!(
                                                            "Actually uses flags from {branch_line}",
                                                        ))
                                                        } else {
                                                            Ok(())
                                                        }
                                                    }
                                                } {
                                                    Ok(()) => {}
                                                    Err(err) => {
                                                        panic!("BRANCH wants to use flags from .USEFLAGS {useflags_line} but: {err}");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        //TODO: should this cause an error?
                                    }
                                }
                                useflags_line = None;
                                code.push(3);
                                code.push(match condition {
                                    crate::assembly::Condition::InputReady => 0,
                                    crate::assembly::Condition::InputNotReady => 1,
                                    crate::assembly::Condition::Equal => 2,
                                    crate::assembly::Condition::NotEqual => 3,
                                    crate::assembly::Condition::Negative => 4,
                                    crate::assembly::Condition::Positive => 5,
                                    crate::assembly::Condition::OverflowSet => 6,
                                    crate::assembly::Condition::OverflowClear => 7,
                                    crate::assembly::Condition::HigherSame => 8,
                                    crate::assembly::Condition::Lower => 9,
                                    crate::assembly::Condition::Higher => 10,
                                    crate::assembly::Condition::LowerSame => 11,
                                    crate::assembly::Condition::GreaterEqual => 12,
                                    crate::assembly::Condition::Less => 13,
                                    crate::assembly::Condition::Greater => 14,
                                    crate::assembly::Condition::LessEqual => 15,
                                });
                                code.label_target(label);
                                code.flush_flags();
                            }
                            crate::assembly::Command::Push(nibble) => {
                                code.push(4);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Pop(nibble) => {
                                code.push(5);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Call(label) => {
                                let target_page = *label_to_page.get(&label).unwrap();
                                if page == target_page {
                                    code.push(6);
                                    code.label_target(label);
                                } else {
                                    match target_page {
                                        PageIdent::Rom(nibble) => {
                                            code.push(12);
                                            code.push(nibble.as_u8());
                                            code.label_target(label);
                                        }
                                        PageIdent::Ram(ident) => {
                                            code.push(1);
                                            code.ram_addr(ident);
                                            code.push(13);
                                            code.label_target(label);
                                        }
                                    }
                                }
                                code.unknown_flags();
                            }
                            crate::assembly::Command::Return => {
                                code.push(7);
                                code.unreachable_flags();
                            }
                            crate::assembly::Command::Add(nibble) => {
                                code.set_flags();
                                code.push(8);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Rotate { shift, register } => {
                                code.push(9);
                                code.push(shift.as_u8());
                                code.push(register.as_u8());
                            }
                            crate::assembly::Command::Duplicate => {
                                code.push(10);
                                code.push(0);
                            }
                            crate::assembly::Command::Not => {
                                code.set_flags();
                                code.push(10);
                                code.push(1);
                            }
                            crate::assembly::Command::Read => {
                                code.push(10);
                                code.push(2);
                            }
                            crate::assembly::Command::ReadPop => {
                                code.push(10);
                                code.push(3);
                            }
                            crate::assembly::Command::Increment => {
                                code.set_flags();
                                code.push(10);
                                code.push(4);
                            }
                            crate::assembly::Command::IncrementWithCarry => {
                                code.set_flags();
                                code.push(10);
                                code.push(5);
                            }
                            crate::assembly::Command::Decrement => {
                                code.set_flags();
                                code.push(10);
                                code.push(6);
                            }
                            crate::assembly::Command::DecrementWithCarry => {
                                code.set_flags();
                                code.push(10);
                                code.push(7);
                            }
                            crate::assembly::Command::Negate => {
                                code.set_flags();
                                code.push(10);
                                code.push(8);
                            }
                            crate::assembly::Command::NegateWithCarry => {
                                code.set_flags();
                                code.push(10);
                                code.push(9);
                            }
                            crate::assembly::Command::NoopSetFlags => {
                                code.set_flags();
                                code.push(10);
                                code.push(10);
                            }
                            crate::assembly::Command::PopSetFlags => {
                                code.set_flags();
                                code.push(10);
                                code.push(11);
                            }
                            crate::assembly::Command::RightShift => {
                                code.set_flags();
                                code.push(10);
                                code.push(12);
                            }
                            crate::assembly::Command::RightShiftCarryIn => {
                                code.set_flags();
                                code.push(10);
                                code.push(13);
                            }
                            crate::assembly::Command::RightShiftOneIn => {
                                code.set_flags();
                                code.push(10);
                                code.push(14);
                            }
                            crate::assembly::Command::ArithmeticRightShift => {
                                code.set_flags();
                                code.push(10);
                                code.push(15);
                            }
                            crate::assembly::Command::Swap(nibble) => {
                                code.push(11);
                                code.push(0);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Sub(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(1);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Write(nibble) => {
                                code.push(11);
                                code.push(2);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::WritePop(nibble) => {
                                code.push(11);
                                code.push(3);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::And(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(4);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Nand(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(5);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Or(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(6);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Nor(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(7);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Xor(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(8);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::NXor(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(9);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::RegToFlags(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(10);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Compare(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(11);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SwapAdd(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(12);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SwapSub(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(13);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::AddWithCarry(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(14);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SubWithCarry(nibble) => {
                                code.set_flags();
                                code.push(11);
                                code.push(15);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::RawRamCall => {
                                code.push(13);
                                code.unknown_flags();
                            }
                            crate::assembly::Command::Input => {
                                code.push(14);
                                code.unknown_flags(); //Not sure what happens here
                            }
                            crate::assembly::Command::Output(vec) => {
                                code.push(15);
                                debug_assert!(!vec.is_empty());
                                for (i, oct) in vec.iter().enumerate() {
                                    let is_last = i + 1 == vec.len();
                                    code.push(
                                        oct.as_u8() | {
                                            match is_last {
                                                false => 0,
                                                true => 8,
                                            }
                                        },
                                    );
                                }
                                // Because the output instruction may pause if the output is blocked, we don't know what the flags will be
                                code.unknown_flags();
                            }
                        },
                        Line::Meta(meta) => match meta {
                            Meta::RomPage(_) => unreachable!(),
                            Meta::RamPage => unreachable!(),
                            Meta::Label(label) => {
                                code.label_location(label);
                                // Because we could jump to here from somewhere else
                                code.unknown_flags();
                            }
                            Meta::UseFlags => match code.flags_as_set {
                                FlagsSetBy::Unreachable => panic!(".USEFLAGS is unreachable"),
                                FlagsSetBy::Unknown => {
                                    panic!(".USEFLAGS has unknown origin for flag")
                                }
                                FlagsSetBy::Nibble(n) => {
                                    useflags_line = Some(n);
                                }
                            },
                            Meta::Comment => {}
                        },
                    }
                }
            }
        }

        let memory = code.finish();
        memory.finish()
    }
}

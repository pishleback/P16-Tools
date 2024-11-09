use std::{collections::HashMap, hash::Hash};

use super::Nibble;
use crate::assembly::{Label, Line, Meta, Program};

#[derive(Debug)]
struct Memory {
    rom_pages: [[Option<super::memory::Nibble>; 256]; 16],
    ram: [Option<super::memory::Nibble>; 1 << (12 + 2)],
}
impl Memory {
    fn blank() -> Self {
        Self {
            rom_pages: [[None; 256]; 16],
            ram: [None; 1 << (12 + 2)],
        }
    }

    fn set_nibble(&mut self, ptr: MemNibblePtr, n: super::memory::Nibble) -> Result<(), ()> {
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

    fn finish(&self) -> super::memory::Memory {
        super::memory::Memory::new(
            core::array::from_fn(|i| {
                core::array::from_fn(|j| {
                    self.rom_pages[i][j].unwrap_or(super::memory::Nibble::new(0))
                })
            }),
            core::array::from_fn(|i| self.ram[i].unwrap_or(super::memory::Nibble::new(0))),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageType {
    Rom(Nibble),
    Ram,
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
impl PageIdent {
    fn as_type(self) -> PageType {
        match self {
            PageIdent::Rom(nibble) => PageType::Rom(nibble),
            PageIdent::Ram(ident) => PageType::Ram,
        }
    }
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
        }
    }
    fn finish(mut self) -> Memory {
        // Replace labels with u8 page addresses
        for (label, blank_page, blank_ptr) in &self.label_targets {
            let (target_page, target_ptr) = self.label_values.get(&label).unwrap();
            let page = blank_page;
            self.memory
                .set_nibble(
                    page.nibble_ptr(*blank_ptr),
                    super::memory::Nibble::new((target_ptr >> 4) & 15),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    page.nibble_ptr(*blank_ptr + 1),
                    super::memory::Nibble::new(target_ptr & 15),
                )
                .unwrap();
        }
        // Replace tagged locations with ram addresses
        for (ident, blank_page, blank_ptr) in &self.ram_addr_targets {
            let addr = *self.ram_ident_to_addr.get(ident).unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 3),
                    super::memory::Nibble::new((addr & 15) as u8),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 2),
                    super::memory::Nibble::new(((addr >> 4) & 15) as u8),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 1),
                    super::memory::Nibble::new(((addr >> 8) & 15) as u8),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr),
                    super::memory::Nibble::new(((addr >> 12) & 15) as u8),
                )
                .unwrap();
        }
        self.memory
    }
}
#[derive(Debug)]
struct MemoryPageManager<'a> {
    memory_manager: &'a mut MemoryManager,
    page: PageLocation,
    ptr: u8,
}
impl<'a> MemoryPageManager<'a> {
    fn nibble_ptr(&self) -> MemNibblePtr {
        match self.page {
            PageLocation::Rom(nibble) => MemNibblePtr::Rom(nibble, self.ptr),
            PageLocation::Ram(base) => MemNibblePtr::Ram(4 * base as usize + self.ptr as usize),
        }
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
    }
    fn push(&mut self, n: u8) {
        self.memory_manager
            .memory
            .set_nibble(self.nibble_ptr(), super::memory::Nibble::new(n))
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

impl Program {
    pub fn compile(&self) -> super::memory::Memory {
        let mut pages: Vec<(PageIdent, Vec<Line>)> = vec![];
        let mut label_to_page: HashMap<Label, PageIdent> = HashMap::new();
        {
            let mut ram_page_ident_counter = 0;
            for line in self.lines() {
                match line {
                    crate::assembly::Line::Meta(Meta::Label(label)) => {
                        if label_to_page.contains_key(&label) {
                            panic!("Duplicate label `{}`", label.to_string());
                        }
                        label_to_page.insert(label.clone(), pages.last().unwrap().0);
                    }
                    _ => {}
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
                                code.push(2);
                                code.label_target(label);
                            }
                            crate::assembly::Command::Branch(condition, label) => {
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
                            }
                            crate::assembly::Command::Return => {
                                code.push(7);
                            }
                            crate::assembly::Command::Add(nibble) => {
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
                                code.push(10);
                                code.push(4);
                            }
                            crate::assembly::Command::IncrementWithCarry => {
                                code.push(10);
                                code.push(5);
                            }
                            crate::assembly::Command::Decrement => {
                                code.push(10);
                                code.push(6);
                            }
                            crate::assembly::Command::DecrementWithCarry => {
                                code.push(10);
                                code.push(7);
                            }
                            crate::assembly::Command::Negate => {
                                code.push(10);
                                code.push(8);
                            }
                            crate::assembly::Command::NegateWithCarry => {
                                code.push(10);
                                code.push(9);
                            }
                            crate::assembly::Command::NoopSetFlags => {
                                code.push(10);
                                code.push(10);
                            }
                            crate::assembly::Command::PopSetFlags => {
                                code.push(10);
                                code.push(11);
                            }
                            crate::assembly::Command::RightShift => {
                                code.push(10);
                                code.push(12);
                            }
                            crate::assembly::Command::RightShiftCarryIn => {
                                code.push(10);
                                code.push(13);
                            }
                            crate::assembly::Command::RightShiftOneIn => {
                                code.push(10);
                                code.push(14);
                            }
                            crate::assembly::Command::ArithmeticRightShift => {
                                code.push(10);
                                code.push(15);
                            }
                            crate::assembly::Command::Swap(nibble) => {
                                code.push(11);
                                code.push(0);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Sub(nibble) => {
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
                                code.push(11);
                                code.push(4);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Nand(nibble) => {
                                code.push(11);
                                code.push(5);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Or(nibble) => {
                                code.push(11);
                                code.push(6);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Nor(nibble) => {
                                code.push(11);
                                code.push(7);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Xor(nibble) => {
                                code.push(11);
                                code.push(8);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::NXor(nibble) => {
                                code.push(11);
                                code.push(9);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::RegToFlags(nibble) => {
                                code.push(11);
                                code.push(10);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::Compare(nibble) => {
                                code.push(11);
                                code.push(11);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SwapAdd(nibble) => {
                                code.push(11);
                                code.push(12);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SwapSub(nibble) => {
                                code.push(11);
                                code.push(13);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::AddWithCarry(nibble) => {
                                code.push(11);
                                code.push(14);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::SubWithCarry(nibble) => {
                                code.push(11);
                                code.push(15);
                                code.push(nibble.as_u8());
                            }
                            crate::assembly::Command::RawRamCall => {
                                code.push(13);
                            }
                            crate::assembly::Command::Input => {
                                code.push(14);
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
                            }
                        },
                        Line::Meta(meta) => match meta {
                            Meta::AutoPage => unreachable!(),
                            Meta::RomPage(_) => unreachable!(),
                            Meta::RamPage => unreachable!(),
                            Meta::Label(label) => {
                                code.label_location(label);
                            }
                            Meta::UseFlags => todo!(),
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

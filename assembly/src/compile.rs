use crate::{
    assembly::{Assembly, Label, Line, Meta},
    WithPos, RAM_SIZE_NIBBLES,
};
use crate::{datatypes::Nibble, ProgramMemory};
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
};

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

    fn finish(&self) -> ProgramMemory {
        ProgramMemory::new(
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
pub enum PageIdent {
    Rom(Nibble),
    Ram(usize),
}

#[derive(Debug)]
struct MemoryManager {
    memory: Memory,
    rom_ptr: [Option<u8>; 16], // None once full
    ram_ptr: Option<usize>,    // None once full
    label_values: HashMap<Label, (PageLocation, u8)>,
    label_targets: Vec<(Label, PageLocation, u8)>,
    ram_ident_to_addr: HashMap<usize, u16>,
    ram_addr_targets: Vec<(usize, PageLocation, u8)>,
}
impl MemoryManager {
    fn blank() -> Self {
        Self {
            memory: Memory::blank(),
            rom_ptr: [Some(0); 16],
            ram_ptr: Some(0),
            label_values: HashMap::new(),
            label_targets: vec![],
            ram_ident_to_addr: HashMap::new(),
            ram_addr_targets: vec![],
        }
    }
    fn new_page<'a>(&'a mut self, page_ident: PageIdent) -> MemoryPageManager<'a> {
        let (page, ptr) = match page_ident {
            PageIdent::Rom(nibble) => {
                if let Some(ptr) = self.rom_ptr[nibble.as_usize()] {
                    (PageLocation::Rom(nibble), Some(ptr))
                } else {
                    (PageLocation::Rom(nibble), None)
                }
            }
            PageIdent::Ram(ident) => {
                if self.ram_ident_to_addr.contains_key(&ident) {
                    panic!("RAM page already added with this identity");
                }

                if let Some(mut ram_ptr) = self.ram_ptr {
                    while !ram_ptr.is_multiple_of(4) {
                        ram_ptr += 1;
                    }
                    if ram_ptr >= RAM_SIZE_NIBBLES as usize {
                        self.ram_ptr = None;
                    } else {
                        self.ram_ptr = Some(ram_ptr);
                    }
                }
                // self.ram_ptr is now on a word boundary
                if let Some(ram_ptr) = self.ram_ptr {
                    let addr = (ram_ptr >> 2) as u16;
                    self.ram_ident_to_addr.insert(ident, addr);
                    (PageLocation::Ram(addr), Some(0))
                } else {
                    (PageLocation::Ram(0), None)
                }
            }
        };
        MemoryPageManager {
            memory_manager: self,
            page,
            page_ident,
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
    // Nothing can set the flags because it is unreachable e.g. RETURN just before
    Unreachable,
    // The flags could come from an unknown source e.g. from a jump from elsewhere
    Unknown,
    // The flags are set here
    // (flags address in page, flags line of assembly)
    Nibble(u8, usize),
}

#[derive(Debug)]
struct MemoryPageManager<'a> {
    memory_manager: &'a mut MemoryManager,
    page: PageLocation,
    page_ident: PageIdent,
    ptr: Option<u8>, // None once full
    // Current start of the flags as
    flags_as_set: FlagsSetBy,
    flags_delay_queue: VecDeque<FlagsSetBy>,
}
impl<'a> MemoryPageManager<'a> {
    fn nibble_ptr(&self) -> MemNibblePtr {
        match self.page {
            PageLocation::Rom(nibble) => MemNibblePtr::Rom(nibble, self.ptr.unwrap()),
            PageLocation::Ram(base) => {
                MemNibblePtr::Ram(4 * base as usize + self.ptr.unwrap() as usize)
            }
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
    fn set_flags(&mut self, line: usize) {
        self.flags_as_set = FlagsSetBy::Nibble(self.ptr.unwrap(), line);
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
                FlagsSetBy::Nibble(l, _) => {
                    if *l == flags_set_on {
                        return Some(i);
                    }
                }
            }
        }
        None
    }
    fn inc(&mut self) {
        if let Some(ptr) = self.ptr {
            // inc this page pointer
            self.ptr = ptr.checked_add(1);
        }

        // inc global pointers
        match self.page {
            PageLocation::Rom(nibble) => {
                if let Some(rom_ptr) = self.memory_manager.rom_ptr[nibble.as_usize()] {
                    match rom_ptr.checked_add(1) {
                        Some(rom_ptr_inc) => {
                            self.memory_manager.rom_ptr[nibble.as_usize()] = Some(rom_ptr_inc);
                        }
                        None => {
                            self.memory_manager.rom_ptr[nibble.as_usize()] = None;
                        }
                    }
                }
            }
            PageLocation::Ram(_) => {
                if let Some(ram_ptr) = self.memory_manager.ram_ptr {
                    let ram_ptr_inc = ram_ptr + 1;
                    if ram_ptr_inc < RAM_SIZE_NIBBLES as usize {
                        self.memory_manager.ram_ptr = Some(ram_ptr_inc);
                    } else {
                        self.ptr = None; // if we run out of RAM then we are full before the 255 nibble page is full
                        self.memory_manager.ram_ptr = None;
                    }
                }
            }
        }

        self.tick_flags_delay();

        // santiy check state is consistent between this page and global
        match self.page {
            PageLocation::Rom(nibble) => {
                debug_assert_eq!(self.memory_manager.rom_ptr[nibble.as_usize()], self.ptr);
            }
            PageLocation::Ram(_) => {
                if self.memory_manager.ram_ptr.is_none() {
                    debug_assert!(self.ptr.is_none());
                }
            }
        }
    }
    fn check_is_full(&self) -> Result<(), CompileError> {
        if self.ptr.is_none() {
            return Err(CompileError::PageFull {
                page: self.page_ident,
            });
        }
        Ok(())
    }
    fn push(&mut self, n: u8) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager
            .memory
            .set_nibble(self.nibble_ptr(), Nibble::new(n).unwrap())
            .unwrap();
        self.inc();
        Ok(())
    }
    fn label_location(&mut self, label: Label) -> Result<(), CompileError> {
        self.check_is_full()?;
        if self.memory_manager.label_values.contains_key(&label) {
            panic!("Label already exists");
        }
        self.memory_manager
            .label_values
            .insert(label.clone(), (self.page, self.ptr.unwrap()));
        Ok(())
    }
    fn label_target(&mut self, label: Label) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager
            .label_targets
            .push((label, self.page, self.ptr.unwrap()));
        self.inc();
        self.check_is_full()?;
        self.inc();
        Ok(())
    }
    fn ram_addr(&mut self, ident: usize) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager
            .ram_addr_targets
            .push((ident, self.page, self.ptr.unwrap()));
        self.inc();
        self.inc();
        self.inc();
        self.check_is_full()?;
        self.inc();
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct LayoutPagesLine {
    line: WithPos<Line>,
    assembly_line_num: usize, //the index of the line in the assembly (not the same as line #)
}

#[derive(Debug, Clone)]
pub struct LayoutPagesSuccess {
    pages: Vec<(PageIdent, Vec<LayoutPagesLine>)>,
    label_to_page: HashMap<Label, PageIdent>,
}

impl LayoutPagesSuccess {
    // The location(s) in the source text of this page as a list of intervals
    pub fn get_page_text_intervals(&self, target_page_ident: &PageIdent) -> Vec<(usize, usize)> {
        match target_page_ident {
            PageIdent::Rom(_) => self
                .pages
                .iter()
                .filter(|(page_ident, _)| page_ident == target_page_ident)
                .collect::<Vec<_>>(),
            PageIdent::Ram(_) => self
                .pages
                .iter()
                .filter(|(page_ident, _)| match page_ident {
                    PageIdent::Rom(_) => false,
                    PageIdent::Ram(_) => true,
                })
                .collect::<Vec<_>>(),
        }
        .into_iter()
        .filter_map(|(_, lines)| {
            if lines.is_empty() {
                None
            } else {
                Some((
                    lines.first().unwrap().line.start,
                    lines.last().unwrap().line.end,
                ))
            }
        })
        .collect()
    }
}

#[derive(Debug, Clone)]
pub enum LayoutPagesError {
    DuplicateLabel { line: usize, label: String },
    MissingPageStart { line: usize },
}

pub fn layout_pages(assembly: &Assembly) -> Result<LayoutPagesSuccess, LayoutPagesError> {
    let mut pages: Vec<(PageIdent, Vec<LayoutPagesLine>)> = vec![];
    let mut label_to_page: HashMap<Label, PageIdent> = HashMap::new();

    let mut ram_page_ident_counter = 0;
    for (line_num, &line) in assembly.lines_with_pos().iter().enumerate() {
        if let crate::assembly::Line::Meta(Meta::Label(label)) = &line.t {
            if label_to_page.contains_key(&label.t) {
                return Err(LayoutPagesError::DuplicateLabel {
                    line: line_num,
                    label: label.t.to_string().clone(),
                });
            }
            label_to_page.insert(label.t.clone(), pages.last().unwrap().0);
        }

        match &line.t {
            crate::assembly::Line::Meta(Meta::RomPage(n)) => {
                pages.push((PageIdent::Rom(n.t), vec![]));
            }
            crate::assembly::Line::Meta(Meta::RamPage) => {
                pages.push((PageIdent::Ram(ram_page_ident_counter), vec![]));
                ram_page_ident_counter += 1;
            }
            _ => match pages.last_mut() {
                Some((_, lines)) => {
                    lines.push(LayoutPagesLine {
                        line: line.clone(),
                        assembly_line_num: line_num,
                    });
                }
                None => {
                    // Probably forgot to specify the first page
                    return Err(LayoutPagesError::MissingPageStart { line: line_num });
                }
            },
        }
    }

    Ok(LayoutPagesSuccess {
        pages,
        label_to_page,
    })
}

#[derive(Debug, Clone)]
pub struct CompileSuccess {
    program_memory: ProgramMemory,
    useflag_lines: HashMap<usize, usize>, // point from .USEFLAG lines to the line whose flags it is using
}

impl CompileSuccess {
    pub fn memory(&self) -> &ProgramMemory {
        &self.program_memory
    }

    pub fn get_useflag_line(&self, useflag_line: usize) -> Option<usize> {
        println!("{:?}", self.useflag_lines);
        self.useflag_lines.get(&useflag_line).cloned()
    }
}

#[derive(Debug, Clone)]
pub enum CompileError {
    Invalid16BitValue {
        line: usize,
    },
    MissingLabel {
        line: usize,
        label: WithPos<Label>,
    },
    JumpOrBranchToOtherPage {
        line: usize,
    },
    BadUseflagsWithBranch {
        branch_line: usize,
        useflags_line: usize,
    },
    BadUseflags {
        useflags_line: usize,
    },
    PageFull {
        page: PageIdent,
    },
}

pub fn compile_assembly(page_layout: &LayoutPagesSuccess) -> Result<CompileSuccess, CompileError> {
    let pages = page_layout.pages.clone();
    let label_to_page = &page_layout.label_to_page;
    let mut useflag_lines = HashMap::new();

    let mut code = MemoryManager::blank();
    for (page, lines) in pages {
        let mut code = code.new_page(page);
        let mut prev_useflag_info: Option<(u8, usize, usize)> = None; // (memory location where we are using flags from, assembly line number where we are using flags from, assembly line number of the .USEFLAGS)
        for LayoutPagesLine {
            line,
            assembly_line_num: line_num,
        } in lines
        {
            match line.t {
                Line::Command(command) => {
                    match command {
                        crate::assembly::Command::Pass => {
                            code.push(0)?;
                        }
                        crate::assembly::Command::Raw(nibbles) => {
                            for nibble in &nibbles.t {
                                code.push(nibble.t.as_u8())?;
                            }
                        }
                        crate::assembly::Command::Value(WithPos { t: v, .. }) => {
                            if v.is_none() {
                                return Err(CompileError::Invalid16BitValue { line: line_num });
                            }
                            let v = v.unwrap();
                            code.push(1)?;
                            let a = (v & 15) as u8;
                            let b = ((v >> 4) & 15) as u8;
                            let c = ((v >> 8) & 15) as u8;
                            let d = ((v >> 12) & 15) as u8;
                            code.push(d)?;
                            code.push(c)?;
                            code.push(b)?;
                            code.push(a)?;
                        }
                        crate::assembly::Command::Jump(label) => {
                            let target_page = label_to_page.get(&label.t);
                            if target_page.is_none() {
                                return Err(CompileError::MissingLabel {
                                    line: line_num,
                                    label,
                                });
                            }
                            if page != *target_page.unwrap() {
                                return Err(CompileError::JumpOrBranchToOtherPage {
                                    line: line_num,
                                });
                            }
                            code.push(2)?;
                            code.label_target(label.t)?;
                        }
                        crate::assembly::Command::Branch(condition, label) => {
                            let target_page = label_to_page.get(&label.t);
                            if target_page.is_none() {
                                return Err(CompileError::MissingLabel {
                                    line: line_num,
                                    label,
                                });
                            }
                            if page != *target_page.unwrap() {
                                return Err(CompileError::JumpOrBranchToOtherPage {
                                    line: line_num,
                                });
                            }
                            match prev_useflag_info {
                                Some((
                                    flags_location,
                                    flags_assembly_line,
                                    useflags_assembly_line,
                                )) => match code.wait_for_flags(flags_location) {
                                    Some(delay) => {
                                        for _ in 0..delay {
                                            code.push(0)?;
                                        }
                                    }
                                    None => {
                                        match match code.delayed_flags_for_branch() {
                                            FlagsSetBy::Unreachable => {
                                                Err("Is unreachable".to_string())
                                            }
                                            FlagsSetBy::Unknown => {
                                                Err("The flags could come from an unknown source"
                                                    .to_string())
                                            }
                                            FlagsSetBy::Nibble(branch_line, _) => {
                                                if flags_location != branch_line {
                                                    Err(format!(
                                                        "Actually uses flags from {branch_line}",
                                                    ))
                                                } else {
                                                    Ok(())
                                                }
                                            }
                                        } {
                                            Ok(()) => {}
                                            Err(_err) => {
                                                return Err(CompileError::BadUseflagsWithBranch {
                                                    branch_line: line_num,
                                                    useflags_line: useflags_assembly_line,
                                                });
                                            }
                                        }
                                    }
                                },
                                None => {
                                    //TODO: should this cause an error? Should every branch require a .USEFLAGS?
                                }
                            }
                            prev_useflag_info = None;
                            code.push(3)?;
                            code.push(match condition.t {
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
                            })?;
                            code.label_target(label.t)?;
                            code.flush_flags();
                        }
                        crate::assembly::Command::Push(nibble) => {
                            code.push(4)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Pop(nibble) => {
                            code.push(5)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Call(label) => {
                            let target_page = label_to_page.get(&label.t);
                            if target_page.is_none() {
                                return Err(CompileError::MissingLabel {
                                    line: line_num,
                                    label,
                                });
                            }
                            let target_page = *target_page.unwrap();
                            if page == target_page {
                                code.push(6)?;
                                code.label_target(label.t)?;
                            } else {
                                match target_page {
                                    PageIdent::Rom(nibble) => {
                                        code.push(12)?;
                                        code.push(nibble.as_u8())?;
                                        code.label_target(label.t)?;
                                    }
                                    PageIdent::Ram(ident) => {
                                        code.push(1)?;
                                        code.ram_addr(ident)?;
                                        code.push(13)?;
                                        code.label_target(label.t)?;
                                    }
                                }
                            }
                            code.unknown_flags();
                        }
                        crate::assembly::Command::Return => {
                            code.push(7)?;
                            code.unreachable_flags();
                        }
                        crate::assembly::Command::Add(nibble) => {
                            code.set_flags(line_num);
                            code.push(8)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Rotate { shift, register } => {
                            code.push(9)?;
                            code.push(shift.t.as_u8())?;
                            code.push(register.t.as_u8())?;
                        }
                        crate::assembly::Command::Duplicate => {
                            code.push(10)?;
                            code.push(0)?;
                        }
                        crate::assembly::Command::Not => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(1)?;
                        }
                        crate::assembly::Command::Read => {
                            code.push(10)?;
                            code.push(2)?;
                        }
                        crate::assembly::Command::ReadPop => {
                            code.push(10)?;
                            code.push(3)?;
                        }
                        crate::assembly::Command::Increment => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(4)?;
                        }
                        crate::assembly::Command::IncrementWithCarry => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(5)?;
                        }
                        crate::assembly::Command::Decrement => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(6)?;
                        }
                        crate::assembly::Command::DecrementWithCarry => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(7)?;
                        }
                        crate::assembly::Command::Negate => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(8)?;
                        }
                        crate::assembly::Command::NegateWithCarry => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(9)?;
                        }
                        crate::assembly::Command::NoopSetFlags => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(10)?;
                        }
                        crate::assembly::Command::PopSetFlags => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(11)?;
                        }
                        crate::assembly::Command::RightShift => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(12)?;
                        }
                        crate::assembly::Command::RightShiftCarryIn => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(13)?;
                        }
                        crate::assembly::Command::RightShiftOneIn => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(14)?;
                        }
                        crate::assembly::Command::ArithmeticRightShift => {
                            code.set_flags(line_num);
                            code.push(10)?;
                            code.push(15)?;
                        }
                        crate::assembly::Command::Swap(nibble) => {
                            code.push(11)?;
                            code.push(0)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Sub(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(1)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Write(nibble) => {
                            code.push(11)?;
                            code.push(2)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::WritePop(nibble) => {
                            code.push(11)?;
                            code.push(3)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::And(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(4)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Nand(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(5)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Or(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(6)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Nor(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(7)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Xor(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(8)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::NXor(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(9)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::RegToFlags(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(10)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::Compare(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(11)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::SwapAdd(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(12)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::SwapSub(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(13)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::AddWithCarry(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(14)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::SubWithCarry(nibble) => {
                            code.set_flags(line_num);
                            code.push(11)?;
                            code.push(15)?;
                            code.push(nibble.t.as_u8())?;
                        }
                        crate::assembly::Command::RawRamCall => {
                            code.push(13)?;
                            code.unknown_flags();
                        }
                        crate::assembly::Command::Input => {
                            code.push(14)?;
                            code.unknown_flags(); //Not sure what happens here
                        }
                        crate::assembly::Command::Output(vec) => {
                            code.push(15)?;
                            debug_assert!(!vec.t.is_empty());
                            for (i, oct) in vec.t.iter().enumerate() {
                                let is_last = i + 1 == vec.t.len();
                                code.push(
                                    oct.t.as_u8() | {
                                        match is_last {
                                            false => 0,
                                            true => 8,
                                        }
                                    },
                                )?;
                            }
                            // Because the output instruction may pause if the output is blocked, we don't know what the flags will be
                            code.unknown_flags();
                        }
                    }
                }
                Line::Meta(meta) => match meta {
                    Meta::RomPage(_) => unreachable!(),
                    Meta::RamPage => unreachable!(),
                    Meta::Label(label) => {
                        code.label_location(label.t)?;
                        // Because we could jump to here from somewhere else
                        code.unknown_flags();
                    }
                    Meta::UseFlags => match code.flags_as_set {
                        FlagsSetBy::Unreachable => {
                            //.USEFLAGS is unreachable
                            return Err(CompileError::BadUseflags {
                                useflags_line: line_num,
                            });
                        }
                        FlagsSetBy::Unknown => {
                            //.USEFLAGS has unknown origin for flag
                            return Err(CompileError::BadUseflags {
                                useflags_line: line_num,
                            });
                        }
                        FlagsSetBy::Nibble(flag_addr, flag_line_num) => {
                            prev_useflag_info = Some((flag_addr, flag_line_num, line_num));
                            useflag_lines.insert(line_num, flag_line_num);
                        }
                    },
                    Meta::Comment(..) => {}
                },
            }
        }
    }

    let memory = code.finish();
    let program_memory = memory.finish();

    Ok(CompileSuccess {
        program_memory,
        useflag_lines,
    })
}

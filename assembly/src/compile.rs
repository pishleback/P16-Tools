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
    ram: [Option<Nibble>; RAM_SIZE_NIBBLES as usize],
}
impl Memory {
    fn blank() -> Self {
        Self {
            rom_pages: [[None; 256]; 16],
            ram: [None; RAM_SIZE_NIBBLES as usize],
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

impl MemNibblePtr {
    fn offset(&self, offset: isize) -> Self {
        match self {
            MemNibblePtr::Rom(page, nibble) => Self::Rom(*page, nibble.wrapping_add(offset as u8)),
            MemNibblePtr::Ram(nibble) => {
                Self::Ram(nibble.wrapping_add(offset as usize) % (RAM_SIZE_NIBBLES as usize))
            }
        }
    }
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
            PageLocation::Ram(base) => MemNibblePtr::Ram(4 * (*base as usize) + a as usize),
        }
    }
}

// A page of the program
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageIdent {
    Rom(Nibble),
    Ram(usize),
}

// A page in assembly
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssemblyPageIdent {
    Data(usize),
    Prog(PageIdent),
}

#[derive(Debug)]
struct MemoryManager {
    memory: Memory,
    rom_ptr: [Option<u8>; 16],     // None once full
    ram_nibble_ptr: Option<usize>, // None once full

    // keep track of labelled locations
    labelled_page_locations: HashMap<Label, (PageLocation, u8)>,
    labelled_ram_addresses: HashMap<Label, u16>,

    // keep track of things which are pointing at labels
    labelled_page_location_targets: Vec<(Label, PageLocation, u8)>,
    ram_page_targets: Vec<(usize, PageLocation, u8)>,
    labelled_ram_address_targets: Vec<(WithPos<Label>, LayoutPagesLine, MemNibblePtr)>,

    // keep track of where each RAM page is in RAM
    ram_ident_to_addr: HashMap<usize, u16>,
}
impl MemoryManager {
    fn blank() -> Self {
        Self {
            memory: Memory::blank(),
            rom_ptr: [Some(0); 16],
            ram_nibble_ptr: Some(0),
            labelled_page_locations: HashMap::new(),
            labelled_ram_addresses: HashMap::new(),
            labelled_page_location_targets: vec![],
            ram_ident_to_addr: HashMap::new(),
            labelled_ram_address_targets: vec![],
            ram_page_targets: vec![],
        }
    }

    fn inc_ram(&mut self) -> bool {
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            let ram_nibble_ptr_inc = ram_nibble_ptr + 1;
            if ram_nibble_ptr_inc < RAM_SIZE_NIBBLES as usize {
                self.ram_nibble_ptr = Some(ram_nibble_ptr_inc);
                true
            } else {
                self.ram_nibble_ptr = None;
                false
            }
        } else {
            false
        }
    }

    // Increase self.rom_ptr until it is at the start of a new 16-bit word and return the address of that word
    // Return None if out of memory
    fn next_ram_word_ptr(&mut self) -> Option<u16> {
        if let Some(mut ram_nibble_ptr) = self.ram_nibble_ptr {
            while !ram_nibble_ptr.is_multiple_of(4) {
                ram_nibble_ptr += 1;
            }
            if ram_nibble_ptr >= RAM_SIZE_NIBBLES as usize {
                self.ram_nibble_ptr = None;
            } else {
                self.ram_nibble_ptr = Some(ram_nibble_ptr);
            }
        }
        self.ram_nibble_ptr
            .map(|ram_nibble_ptr| (ram_nibble_ptr / 4) as u16)
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
                self.next_ram_word_ptr();
                // self.ram_ptr is now on a word boundary
                if let Some(ram_ptr) = self.ram_nibble_ptr {
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
            flags_as_set: FlagsState { sources: vec![] },
            flags_delay_queue: (0..6).map(|_i| FlagsState { sources: vec![] }).collect(),
        }
    }

    fn push_ram(&mut self, value: u16) -> Result<(), CompileError> {
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            self.memory
                .set_nibble(
                    MemNibblePtr::Ram(ram_nibble_ptr),
                    Nibble::new(((value >> 12) & 15) as u8).unwrap(),
                )
                .unwrap();
        } else {
            return Err(CompileError::RamFull);
        }
        self.inc_ram();
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            self.memory
                .set_nibble(
                    MemNibblePtr::Ram(ram_nibble_ptr),
                    Nibble::new(((value >> 8) & 15) as u8).unwrap(),
                )
                .unwrap();
        } else {
            return Err(CompileError::RamFull);
        }
        self.inc_ram();
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            self.memory
                .set_nibble(
                    MemNibblePtr::Ram(ram_nibble_ptr),
                    Nibble::new(((value >> 4) & 15) as u8).unwrap(),
                )
                .unwrap();
        } else {
            return Err(CompileError::RamFull);
        }
        self.inc_ram();
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            self.memory
                .set_nibble(
                    MemNibblePtr::Ram(ram_nibble_ptr),
                    Nibble::new((value & 15) as u8).unwrap(),
                )
                .unwrap();
        } else {
            return Err(CompileError::RamFull);
        }
        self.inc_ram();
        Ok(())
    }

    fn label_ram_address(
        &mut self,
        label: WithPos<Label>,
        line: LayoutPagesLine,
    ) -> Result<(), CompileError> {
        if let Some(ram_ptr) = self.next_ram_word_ptr() {
            if self.labelled_ram_addresses.contains_key(&label.t) {
                return Err(CompileError::DuplicateRamLabel {
                    line: line.assembly_line_num,
                    label,
                });
            }
            self.labelled_ram_addresses.insert(label.t.clone(), ram_ptr);
            Ok(())
        } else {
            Err(CompileError::RamFull)
        }
    }

    fn push_labelled_ram_address(
        &mut self,
        label: WithPos<Label>,
        line: LayoutPagesLine,
    ) -> Result<(), CompileError> {
        if let Some(ram_nibble_ptr) = self.ram_nibble_ptr {
            self.labelled_ram_address_targets.push((
                label,
                line,
                MemNibblePtr::Ram(ram_nibble_ptr),
            ));
            self.inc_ram();
            self.inc_ram();
            self.inc_ram();
            if self.ram_nibble_ptr.is_none() {
                return Err(CompileError::RamFull);
            }
            self.inc_ram();
            Ok(())
        } else {
            Err(CompileError::RamFull)
        }
    }

    fn finish(mut self) -> Result<Memory, CompileError> {
        // Replace labels with u8 page addresses
        for (label, blank_page, blank_ptr) in &self.labelled_page_location_targets {
            let (_target_page, target_ptr) = self.labelled_page_locations.get(label).unwrap();

            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr),
                    Nibble::new((target_ptr >> 4) & 15).unwrap(),
                )
                .unwrap();
            self.memory
                .set_nibble(
                    blank_page.nibble_ptr(*blank_ptr + 1),
                    Nibble::new(target_ptr & 15).unwrap(),
                )
                .unwrap();
        }
        // Replace tagged locations with ram addresses
        for (ident, blank_page, blank_ptr) in &self.ram_page_targets {
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
        // Replace RAM labels with addresses
        for (label, line, blank_nibble_ptr) in &self.labelled_ram_address_targets {
            if let Some(address) = self.labelled_ram_addresses.get(&label.t).cloned() {
                self.memory
                    .set_nibble(
                        blank_nibble_ptr.clone(),
                        Nibble::new(((address >> 12) & 15) as u8).unwrap(),
                    )
                    .unwrap();
                self.memory
                    .set_nibble(
                        blank_nibble_ptr.offset(1),
                        Nibble::new(((address >> 8) & 15) as u8).unwrap(),
                    )
                    .unwrap();
                self.memory
                    .set_nibble(
                        blank_nibble_ptr.offset(2),
                        Nibble::new(((address >> 4) & 15) as u8).unwrap(),
                    )
                    .unwrap();
                self.memory
                    .set_nibble(
                        blank_nibble_ptr.offset(3),
                        Nibble::new((address & 15) as u8).unwrap(),
                    )
                    .unwrap();
            } else {
                return Err(CompileError::MissingRamLabel {
                    line: line.assembly_line_num,
                    label: label.clone(),
                });
            }
        }

        Ok(self.memory)
    }
}

// represent the possible states of part of the flag bus at a given moment in time

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlagsState {
    // List of places whre the flags could have been set
    // Vec<(flags address in page, flags line of assembly)>
    // In the order in which they appear
    sources: Vec<(u8, usize)>,
}

#[derive(Debug)]
struct MemoryPageManager<'a> {
    memory_manager: &'a mut MemoryManager,
    page: PageLocation,
    page_ident: PageIdent,
    ptr: Option<u8>, // None once full
    // Current start of the flags as
    flags_as_set: FlagsState,
    // .front() is straight out the ALU and .back() is as seen by a branch
    flags_delay_queue: VecDeque<FlagsState>,
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
    fn current_flags_branch_pov(&self) -> FlagsState {
        self.flags_delay_queue.back().unwrap().clone()
    }
    fn tick_flags_delay(&mut self) {
        self.flags_delay_queue.push_front(self.flags_as_set.clone());
        self.flags_delay_queue.pop_back().unwrap();
    }
    fn flush_flags(&mut self) {
        self.flags_delay_queue = self
            .flags_delay_queue
            .iter()
            .map(|_f| self.flags_as_set.clone())
            .collect();
    }
    // overwrite the current flags out of the ALU
    fn set_flags(&mut self, line: usize) {
        self.flags_as_set = FlagsState {
            sources: vec![(self.ptr.unwrap(), line)],
        }
    }
    // populate the flag queue with flags which could also be set here
    fn set_possible_flushed_flags(&mut self, line: usize) -> Result<(), CompileError> {
        self.check_is_full()?;
        let flag_state = (self.ptr.unwrap(), line);
        self.flags_as_set.sources.push(flag_state);
        for entry in &mut self.flags_delay_queue {
            entry.sources.push(flag_state);
        }
        Ok(())
    }
    fn unreachable_flags(&mut self) {
        self.flags_as_set = FlagsState { sources: vec![] };
        self.flush_flags();
    }
    // how much delay must be added for flags set at <flags_set_on> to be useable in a branch instruction
    // return None if no ammount of delay will do the trick, e.g. if something else has long since overwritten the flags
    fn wait_for_flags(&mut self, flags: &FlagsState) -> Option<usize> {
        for (i, flags_after_i) in self
            .flags_delay_queue
            .iter()
            .rev()
            .chain(vec![&self.flags_as_set])
            .enumerate()
        {
            if flags == flags_after_i {
                return Some(i);
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
                if !self.memory_manager.inc_ram() {
                    self.ptr = None; // if we run out of RAM then we are full before the 255 nibble page is full
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
                if self.memory_manager.ram_nibble_ptr.is_none() {
                    debug_assert!(self.ptr.is_none());
                }
            }
        }
    }
    fn check_is_full(&self) -> Result<(), CompileError> {
        if self.ptr.is_none() {
            return Err(match self.page_ident {
                PageIdent::Rom(nibble) => CompileError::RomPageFull { page: nibble },
                PageIdent::Ram(_) => CompileError::RamFull,
            });
        }
        Ok(())
    }
    // label a position in the program page
    fn label_page_location(&mut self, label: Label) -> Result<(), CompileError> {
        self.check_is_full()?;
        if self
            .memory_manager
            .labelled_page_locations
            .contains_key(&label)
        {
            panic!("Label already exists");
        }
        self.memory_manager
            .labelled_page_locations
            .insert(label.clone(), (self.page, self.ptr.unwrap()));
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
    fn push_labelled_page_location(&mut self, label: Label) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager.labelled_page_location_targets.push((
            label,
            self.page,
            self.ptr.unwrap(),
        ));
        self.inc();
        self.check_is_full()?;
        self.inc();
        Ok(())
    }
    fn push_page_ram_addr(&mut self, ram_page_ident: usize) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager
            .ram_page_targets
            .push((ram_page_ident, self.page, self.ptr.unwrap()));
        self.inc();
        self.inc();
        self.inc();
        self.check_is_full()?;
        self.inc();
        Ok(())
    }
    fn push_labelled_ram_address(
        &mut self,
        label: WithPos<Label>,
        line: LayoutPagesLine,
    ) -> Result<(), CompileError> {
        self.check_is_full()?;
        self.memory_manager.labelled_ram_address_targets.push((
            label,
            line,
            self.page.nibble_ptr(self.ptr.unwrap()),
        ));
        self.inc();
        self.inc();
        self.inc();
        self.check_is_full()?;
        self.inc();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LayoutPagesLine {
    pub line: WithPos<Line>,
    pub assembly_line_num: usize, //the index of the line in the assembly (not the same as line #)
}

#[derive(Debug, Clone)]
pub struct LayoutPagesSuccess {
    pages: Vec<(AssemblyPageIdent, Vec<LayoutPagesLine>)>,
    label_to_page: HashMap<Label, PageIdent>,
}

impl LayoutPagesSuccess {
    // The location(s) in the source text of this page as a list of intervals
    pub fn get_ram_text_intervals(&self) -> Vec<(usize, usize)> {
        self.pages
            .iter()
            .filter(|(page_ident, _)| match page_ident {
                AssemblyPageIdent::Prog(PageIdent::Rom(_)) => false,
                AssemblyPageIdent::Prog(PageIdent::Ram(_)) => true,
                AssemblyPageIdent::Data(_) => true,
            })
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

    pub fn get_rom_page_text_intervals(&self, page: Nibble) -> Vec<(usize, usize)> {
        self.pages
            .iter()
            .filter(|(page_ident, _)| page_ident == &AssemblyPageIdent::Prog(PageIdent::Rom(page)))
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
    let mut pages: Vec<(AssemblyPageIdent, Vec<LayoutPagesLine>)> = vec![];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CurrentSection {
        Unset,
        Prog(usize), // index in pages
    }
    let mut current_section = CurrentSection::Unset;

    let mut label_to_page: HashMap<Label, PageIdent> = HashMap::new();

    let mut ram_page_ident_counter = 0;
    let mut data_page_ident_counter = 0;
    for (line_num, &line) in assembly.lines_with_pos().iter().enumerate() {
        if let crate::assembly::Line::Meta(Meta::Label(label)) = &line.t {
            match current_section {
                CurrentSection::Unset => {
                    return Err(LayoutPagesError::MissingPageStart { line: line_num });
                }
                CurrentSection::Prog(idx) => match pages[idx].0 {
                    AssemblyPageIdent::Data(_) => {}
                    AssemblyPageIdent::Prog(page) => {
                        if label_to_page.contains_key(&label.t) {
                            return Err(LayoutPagesError::DuplicateLabel {
                                line: line_num,
                                label: label.t.to_string().clone(),
                            });
                        }
                        label_to_page.insert(label.t.clone(), page);
                    }
                },
            }
        }

        match &line.t {
            crate::assembly::Line::Meta(Meta::RomPage(n)) => {
                current_section = CurrentSection::Prog(pages.len());
                pages.push((AssemblyPageIdent::Prog(PageIdent::Rom(n.t)), vec![]));
            }
            crate::assembly::Line::Meta(Meta::RamPage) => {
                current_section = CurrentSection::Prog(pages.len());
                pages.push((
                    AssemblyPageIdent::Prog(PageIdent::Ram(ram_page_ident_counter)),
                    vec![],
                ));
                ram_page_ident_counter += 1;
            }
            crate::assembly::Line::Meta(Meta::Data) => {
                current_section = CurrentSection::Prog(pages.len());
                pages.push((AssemblyPageIdent::Data(data_page_ident_counter), vec![]));
                data_page_ident_counter += 1;
            }
            _ => {
                let lines = match current_section {
                    CurrentSection::Unset => {
                        return Err(LayoutPagesError::MissingPageStart { line: line_num });
                    }
                    CurrentSection::Prog(idx) => &mut pages[idx].1,
                };
                lines.push(LayoutPagesLine {
                    line: line.clone(),
                    assembly_line_num: line_num,
                });
            }
        }
    }

    Ok(LayoutPagesSuccess {
        pages,
        label_to_page,
    })
}

#[derive(Debug, Clone)]
pub struct CompiledLine {
    pub line: WithPos<Line>,
    pub assembly_line_num: usize, //the index of the line in the assembly (not the same as line #)
    // where in the program page does it appear
    pub page_start: usize,
    pub page_end: usize,
}

// The location in RAM of a ..RAM page
#[derive(Debug, Clone)]
pub struct RamPageLocation {
    pub start: u16,
    pub length: u16,
}

#[derive(Debug, Clone)]
pub struct CompileSuccess {
    program_memory: ProgramMemory,
    ram_pages: Vec<RamPageLocation>, // One for each PageIdent::Ram(#) in the page layout i.e. one for each ..RAM section in the assembly in the same order as they appear
    rom_lines: [Vec<CompiledLine>; 16],
    ram_lines: Vec<Vec<CompiledLine>>, // outer vec bijects with the ..RAM pages
    useflag_lines: HashMap<usize, Vec<usize>>, // point from .USEFLAG lines to the line whose flags it could be using
    branch_lines: HashMap<usize, usize>,       // point from BRANCH to the .USEFLAG line it is using
}

impl CompileSuccess {
    pub fn memory(&self) -> &ProgramMemory {
        &self.program_memory
    }

    pub fn ram_pages(&self) -> Vec<RamPageLocation> {
        self.ram_pages.clone()
    }

    pub fn rom_lines(&self, page: Nibble) -> &Vec<CompiledLine> {
        &self.rom_lines[page.as_usize()]
    }

    pub fn ram_lines(&self, ident: usize) -> &Vec<CompiledLine> {
        &self.ram_lines[ident]
    }

    pub fn flag_setters_from_useflag(&self, useflag_line: usize) -> Option<Vec<usize>> {
        self.useflag_lines.get(&useflag_line).cloned()
    }

    pub fn useflag_from_branch(&self, branch_line: usize) -> Option<usize> {
        self.branch_lines.get(&branch_line).cloned()
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
    MissingRamLabel {
        line: usize,
        label: WithPos<Label>,
    },
    DuplicateRamLabel {
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
    // BranchWithoutUseflags {
    //     branch_line: usize,
    // },
    RomPageFull {
        page: Nibble,
    },
    InvalidCommandLocation {
        line: usize,
    },
    RamFull,
}

pub fn compile_assembly(page_layout: &LayoutPagesSuccess) -> Result<CompileSuccess, CompileError> {
    let pages = page_layout.pages.clone();
    let label_to_page = &page_layout.label_to_page;

    let mut ram_pages = vec![];
    let mut rom_lines: [Vec<CompiledLine>; 16] = Default::default();
    let mut ram_lines = vec![];
    let mut useflag_lines: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut branch_lines: HashMap<usize, usize> = HashMap::new();

    let mut code = MemoryManager::blank();
    for (page, lines) in pages {
        match page {
            AssemblyPageIdent::Prog(page) => {
                let mut code = code.new_page(page);
                let mut useflag_saved_flag_state: Option<(FlagsState, usize)> = None; // (place where flags were set, assembly line of .USEFLAGS)
                let mut page_lines = vec![];
                for line in lines {
                    let start = code.ptr.map(|p| p as usize).unwrap_or(256);

                    match line.line.t.clone() {
                        Line::Command(command) => {
                            match command {
                                crate::assembly::Command::Pass => {
                                    code.push(0)?;
                                }
                                crate::assembly::Command::Raw(nibbles) => {
                                    code.set_possible_flushed_flags(line.assembly_line_num)?;
                                    for nibble in &nibbles.t {
                                        code.push(nibble.t.as_u8())?;
                                    }
                                }
                                crate::Command::RawLabel(label) => {
                                    code.set_possible_flushed_flags(line.assembly_line_num)?;
                                    let target_page = label_to_page.get(&label.t);
                                    if target_page.is_none() {
                                        return Err(CompileError::MissingLabel {
                                            line: line.assembly_line_num,
                                            label,
                                        });
                                    }
                                    code.push_labelled_page_location(label.t)?;
                                }
                                crate::assembly::Command::Value(WithPos { t: v, .. }) => {
                                    if v.is_none() {
                                        return Err(CompileError::Invalid16BitValue {
                                            line: line.assembly_line_num,
                                        });
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
                                crate::Command::AddressValue(label) => {
                                    code.push(1)?;
                                    code.push_labelled_ram_address(label, line.clone())?;
                                }
                                crate::assembly::Command::Jump(label) => {
                                    code.unreachable_flags();
                                    let target_page = label_to_page.get(&label.t);
                                    if target_page.is_none() {
                                        return Err(CompileError::MissingLabel {
                                            line: line.assembly_line_num,
                                            label,
                                        });
                                    }
                                    if page != *target_page.unwrap() {
                                        return Err(CompileError::JumpOrBranchToOtherPage {
                                            line: line.assembly_line_num,
                                        });
                                    }
                                    code.push(2)?;
                                    code.push_labelled_page_location(label.t)?;
                                }
                                crate::assembly::Command::Branch(condition, label) => {
                                    let target_page = label_to_page.get(&label.t);
                                    if target_page.is_none() {
                                        return Err(CompileError::MissingLabel {
                                            line: line.assembly_line_num,
                                            label,
                                        });
                                    }
                                    if page != *target_page.unwrap() {
                                        return Err(CompileError::JumpOrBranchToOtherPage {
                                            line: line.assembly_line_num,
                                        });
                                    }
                                    match useflag_saved_flag_state {
                                        Some((flags, useflags_assembly_line)) => {
                                            match code.wait_for_flags(&flags) {
                                                Some(delay) => {
                                                    for _ in 0..delay {
                                                        code.push(0)?;
                                                    }
                                                }
                                                None => {
                                                    let current_branch_flags =
                                                        code.current_flags_branch_pov();
                                                    if flags != current_branch_flags {
                                                        return Err(
                                                            CompileError::BadUseflagsWithBranch {
                                                                branch_line: line.assembly_line_num,
                                                                useflags_line:
                                                                    useflags_assembly_line,
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                            branch_lines.insert(
                                                line.assembly_line_num,
                                                useflags_assembly_line,
                                            );
                                        }
                                        None => {
                                            // all branches require a .USEFLAGS first
                                            // return Err(CompileError::BranchWithoutUseflags {
                                            //     branch_line: line_num,
                                            // });
                                        }
                                    }
                                    // debug_assert!(branch_lines.contains_key(&line_num));
                                    useflag_saved_flag_state = None;
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
                                    code.push_labelled_page_location(label.t)?;
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
                                    code.set_possible_flushed_flags(line.assembly_line_num)?;
                                    code.flush_flags();
                                    let target_page = label_to_page.get(&label.t);
                                    if target_page.is_none() {
                                        return Err(CompileError::MissingLabel {
                                            line: line.assembly_line_num,
                                            label,
                                        });
                                    }
                                    let target_page = *target_page.unwrap();
                                    if page == target_page {
                                        code.push(6)?;
                                        code.push_labelled_page_location(label.t)?;
                                    } else {
                                        match target_page {
                                            PageIdent::Rom(nibble) => {
                                                code.push(12)?;
                                                code.push(nibble.as_u8())?;
                                                code.push_labelled_page_location(label.t)?;
                                            }
                                            PageIdent::Ram(ident) => {
                                                code.push(1)?;
                                                code.push_page_ram_addr(ident)?;
                                                code.push(13)?;
                                                code.push_labelled_page_location(label.t)?;
                                            }
                                        }
                                    }
                                }
                                crate::assembly::Command::Return => {
                                    code.unreachable_flags();
                                    code.push(7)?;
                                }
                                crate::assembly::Command::Add(nibble) => {
                                    code.set_flags(line.assembly_line_num);
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
                                    code.set_flags(line.assembly_line_num);
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
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(4)?;
                                }
                                crate::assembly::Command::IncrementWithCarry => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(5)?;
                                }
                                crate::assembly::Command::Decrement => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(6)?;
                                }
                                crate::assembly::Command::DecrementWithCarry => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(7)?;
                                }
                                crate::assembly::Command::Negate => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(8)?;
                                }
                                crate::assembly::Command::NegateWithCarry => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(9)?;
                                }
                                crate::assembly::Command::NoopSetFlags => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(10)?;
                                }
                                crate::assembly::Command::PopSetFlags => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(11)?;
                                }
                                crate::assembly::Command::RightShift => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(12)?;
                                }
                                crate::assembly::Command::RightShiftCarryIn => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(13)?;
                                }
                                crate::assembly::Command::RightShiftOneIn => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(14)?;
                                }
                                crate::assembly::Command::ArithmeticRightShift => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(10)?;
                                    code.push(15)?;
                                }
                                crate::assembly::Command::Swap(nibble) => {
                                    code.push(11)?;
                                    code.push(0)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Sub(nibble) => {
                                    code.set_flags(line.assembly_line_num);
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
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(4)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Nand(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(5)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Or(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(6)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Nor(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(7)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Xor(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(8)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::NXor(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(9)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::RegToFlags(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(10)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::Compare(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(11)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::SwapAdd(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(12)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::SwapSub(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(13)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::AddWithCarry(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(14)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::SubWithCarry(nibble) => {
                                    code.set_flags(line.assembly_line_num);
                                    code.push(11)?;
                                    code.push(15)?;
                                    code.push(nibble.t.as_u8())?;
                                }
                                crate::assembly::Command::RawRamCall => {
                                    code.set_possible_flushed_flags(line.assembly_line_num)?;
                                    code.flush_flags();
                                    code.push(13)?;
                                }
                                crate::assembly::Command::Input => {
                                    code.push(14)?;
                                    // In terms of flags, we can progress them by some fixed ammount since Input always takes some delay. Not sure how much that amount is without checking in game. It could be a full flush?
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
                                    // Can't flush flags. Output instructions may or may not block.
                                }
                                crate::Command::Alloc(_) => {
                                    return Err(CompileError::InvalidCommandLocation {
                                        line: line.assembly_line_num,
                                    });
                                }
                            }
                        }
                        Line::Meta(meta) => match meta {
                            Meta::RomPage(_) => unreachable!(),
                            Meta::RamPage => unreachable!(),
                            Meta::Data => unreachable!(),
                            Meta::Label(label) => {
                                code.set_possible_flushed_flags(line.assembly_line_num)?; // something could goto here so we don't know what the flags are now
                                code.label_page_location(label.t)?;
                            }
                            Meta::UseFlags => {
                                useflag_saved_flag_state =
                                    Some((code.flags_as_set.clone(), line.assembly_line_num));
                                useflag_lines.insert(
                                    line.assembly_line_num,
                                    code.flags_as_set.sources.iter().map(|x| x.1).collect(),
                                );
                            }
                        },
                    }

                    let end = code.ptr.map(|p| p as usize).unwrap_or(256);
                    page_lines.push(CompiledLine {
                        assembly_line_num: line.assembly_line_num,
                        line: line.line,
                        page_start: start,
                        page_end: end,
                    });
                }

                // record where the lines are in raw memory
                match page {
                    PageIdent::Rom(page) => {
                        rom_lines[page.as_usize()].extend(page_lines);
                    }
                    PageIdent::Ram(ident) => {
                        debug_assert_eq!(ident, ram_lines.len());
                        ram_lines.push(page_lines);
                    }
                }

                // update ram_pages with the location of that page if it's a RAM page
                match code.page_ident {
                    PageIdent::Rom(_) => {}
                    PageIdent::Ram(ident) => {
                        debug_assert_eq!(ident, ram_pages.len());
                        match code.page {
                            PageLocation::Rom(_) => {
                                unreachable!()
                            }
                            PageLocation::Ram(start) => {
                                let length = code.ptr.map(|x| x as u16).unwrap_or(256);
                                ram_pages.push(RamPageLocation { start, length });
                            }
                        }
                    }
                }
            }
            AssemblyPageIdent::Data(_) => {
                for line in lines {
                    match line.line.t.clone() {
                        Line::Command(command) => match command {
                            crate::Command::Value(WithPos { t: v, .. }) => {
                                if v.is_none() {
                                    return Err(CompileError::Invalid16BitValue {
                                        line: line.assembly_line_num,
                                    });
                                }
                                code.push_ram(v.unwrap())?;
                            }
                            crate::Command::AddressValue(label) => {
                                code.push_labelled_ram_address(label, line.clone())?;
                            }
                            crate::Command::Alloc(v) => {
                                if v.t.is_none() {
                                    return Err(CompileError::Invalid16BitValue {
                                        line: line.assembly_line_num,
                                    });
                                }
                                let quantity = v.t.unwrap();
                                for _ in 0..quantity {
                                    code.push_ram(0)?;
                                }
                            }
                            _ => {
                                return Err(CompileError::InvalidCommandLocation {
                                    line: line.assembly_line_num,
                                });
                            }
                        },
                        Line::Meta(meta) => match meta {
                            Meta::RomPage(_) | Meta::RamPage | Meta::Data => unreachable!(),
                            Meta::Label(label) => {
                                code.label_ram_address(label, line)?;
                            }
                            Meta::UseFlags => {
                                return Err(CompileError::InvalidCommandLocation {
                                    line: line.assembly_line_num,
                                });
                            }
                        },
                    }
                }
            }
        }
    }

    let memory = code.finish()?;
    let program_memory = memory.finish();

    Ok(CompileSuccess {
        program_memory,
        ram_pages,
        rom_lines,
        ram_lines,
        useflag_lines,
        branch_lines,
    })
}

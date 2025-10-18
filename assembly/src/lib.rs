mod assembly;
mod compile;
mod datatypes;
mod memory;
mod simulator;

pub use assembly::load_assembly;
pub use assembly::Assembly;
pub use assembly::Command;
pub use assembly::Condition;
pub use assembly::Label;
pub use assembly::Line;
pub use assembly::Meta;
pub use assembly::WithPos;
pub use compile::compile_assembly;
pub use compile::layout_pages;
pub use compile::CompileError;
pub use compile::CompileSuccess;
pub use compile::CompiledLine;
pub use compile::LayoutPagesError;
pub use compile::LayoutPagesLine;
pub use compile::LayoutPagesSuccess;
pub use compile::PageIdent;
pub use datatypes::Nibble;
pub use datatypes::OctDigit;
pub use memory::ProgramMemory;
pub use memory::RamMem;
pub use memory::RAM_SIZE;
pub use memory::RAM_SIZE_NIBBLES;
pub use simulator::EndErrorState;
pub use simulator::EndStepOkState;
pub use simulator::ProgramPagePtr;
pub use simulator::ProgramPtr;
pub use simulator::Simulator;

pub type FullCompileResult<'a> = Result<
    (
        Result<(Result<CompileSuccess, CompileError>, LayoutPagesSuccess), LayoutPagesError>,
        Assembly,
    ),
    lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'a>, &'static str>,
>;

pub fn full_compile(text: &str) -> FullCompileResult<'_> {
    load_assembly(text).map(|assembly| {
        (
            layout_pages(&assembly)
                .map(|page_layout| (compile_assembly(&page_layout), page_layout)),
            assembly,
        )
    })
}

use std::collections::HashSet;

use crate::app::state::State;
use assembly::{Command, FullCompileResult, Label, WithPos};
use btree_range_map::RangeMap;
use egui::{Color32, Stroke, TextBuffer, TextFormat, Visuals, text::LayoutJob};

pub fn update(
    state: &mut State,
    compile_result: &FullCompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    let selected_lines = state.selected_lines.clone();

    // Define layouter closure
    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job(text.as_str(), &selected_lines, compile_result, ui.visuals());
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .stick_to_bottom(false)
        .max_height(600.0)
        .show(ui, |ui| {
            let output = egui::TextEdit::multiline(&mut state.source)
                .id("assembly-text-area".into())
                .font(egui::TextStyle::Monospace)
                .desired_rows(20)
                .lock_focus(true)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui);

            // select lines of assembly based on what is highlighted
            match compile_result {
                Ok((_, assembly)) => {
                    if let Some(cursor_range) = output.cursor_range {
                        let cursor_range = cursor_range.sorted_cursors();
                        let (a, b) = (cursor_range[0].index, cursor_range[1].index);
                        debug_assert!(a <= b);
                        let mut selected_lines = HashSet::new();
                        for (line_num, line) in assembly.lines_with_pos().into_iter().enumerate() {
                            if line.start <= b && a <= line.end {
                                selected_lines.insert(line_num);
                            }
                        }
                        state.selected_lines = Some(selected_lines);
                    } else {
                        state.selected_lines = None;
                    }
                }
                Err(_) => {
                    state.selected_lines = None;
                }
            }
        });
}

#[derive(Default, Debug)]
struct TextAttrs {
    colour: RangeMap<usize, Color32>,
    underline: RangeMap<usize, Stroke>,
    italics: RangeMap<usize, bool>,
}

fn layout_job(
    text: &str,
    selected_lines: &Option<HashSet<usize>>,
    result: &FullCompileResult,
    visuals: &Visuals,
) -> LayoutJob {
    let mut text_attrs = TextAttrs::default();

    let red_underline = Stroke {
        width: 1.5,
        color: Color32::RED,
    };

    let purple_underline = Stroke {
        width: 1.5,
        color: Color32::PURPLE,
    };

    match result {
        Ok((result, assembly)) => {
            // Parse success; apply colouring to text
            for WithPos {
                start,
                end,
                t: line,
            } in assembly.lines_with_pos()
            {
                // opperation code
                text_attrs
                    .colour
                    .insert(*start..*end, visuals.strong_text_color());

                // labels
                let add_label = |text_attrs: &mut TextAttrs, label: &WithPos<Label>| {
                    text_attrs.colour.insert(
                        label.start..label.end,
                        visuals.text_color().lerp_to_gamma(Color32::GREEN, 0.5),
                    );
                };

                // registers
                let add_register =
                    |text_attrs: &mut TextAttrs, register: &WithPos<assembly::Nibble>| {
                        text_attrs.colour.insert(
                            register.start..register.end,
                            visuals.text_color().lerp_to_gamma(Color32::YELLOW, 0.5),
                        );
                    };

                // syntax highlighting
                match line {
                    assembly::Line::Command(command) => match command {
                        assembly::Command::Raw(n) => {
                            text_attrs.colour.insert(
                                n.start..n.end,
                                visuals.text_color().lerp_to_gamma(Color32::CYAN, 0.5),
                            );
                        }
                        assembly::Command::Value(v) => {
                            text_attrs.colour.insert(
                                v.start..v.end,
                                visuals.text_color().lerp_to_gamma(Color32::CYAN, 0.5),
                            );
                        }
                        assembly::Command::Push(register)
                        | assembly::Command::Pop(register)
                        | assembly::Command::Add(register)
                        | assembly::Command::Swap(register)
                        | assembly::Command::Sub(register)
                        | assembly::Command::Write(register)
                        | assembly::Command::WritePop(register)
                        | assembly::Command::And(register)
                        | assembly::Command::Nand(register)
                        | assembly::Command::Or(register)
                        | assembly::Command::Nor(register)
                        | assembly::Command::Xor(register)
                        | assembly::Command::NXor(register)
                        | assembly::Command::RegToFlags(register)
                        | assembly::Command::Compare(register)
                        | assembly::Command::SwapAdd(register)
                        | assembly::Command::SwapSub(register)
                        | assembly::Command::AddWithCarry(register)
                        | assembly::Command::SubWithCarry(register) => {
                            add_register(&mut text_attrs, register);
                        }
                        assembly::Command::Jump(label) => {
                            add_label(&mut text_attrs, label);
                        }
                        assembly::Command::Branch(condition, label) => {
                            text_attrs.colour.insert(
                                condition.start..condition.end,
                                visuals.text_color().lerp_to_gamma(Color32::BROWN, 0.5),
                            );
                            add_label(&mut text_attrs, label);
                        }
                        assembly::Command::Call(label) => {
                            add_label(&mut text_attrs, label);
                        }
                        assembly::Command::Rotate { shift, register } => {
                            text_attrs.colour.insert(
                                shift.start..shift.end,
                                visuals.text_color().lerp_to_gamma(Color32::CYAN, 0.5),
                            );
                            add_register(&mut text_attrs, register);
                        }
                        assembly::Command::Output(path) => {
                            text_attrs.colour.insert(
                                path.start..path.end,
                                visuals.text_color().lerp_to_gamma(Color32::CYAN, 0.5),
                            );
                        }
                        _ => {}
                    },
                    assembly::Line::Meta(meta) => match meta {
                        assembly::Meta::Label(label) => {
                            text_attrs.italics.insert(*start..label.end, true);
                            add_label(&mut text_attrs, label);
                        }
                        assembly::Meta::RomPage(page) => {
                            text_attrs.italics.insert(*start..page.end, true);
                            text_attrs
                                .colour
                                .insert(page.start..page.end, visuals.strong_text_color());
                        }
                        assembly::Meta::RamPage => {
                            text_attrs.italics.insert(*start..*end, true);
                        }
                        assembly::Meta::UseFlags => {
                            text_attrs.italics.insert(*start..*end, true);
                        }
                        assembly::Meta::Comment(comment) => {
                            text_attrs.italics.insert(*start..comment.end, true);
                        }
                    },
                }
            }

            match result {
                Ok((result, page_layout)) => match result {
                    Ok(compiled) => {
                        // extra highlighting for selections
                        if let Some(selected_lines) = selected_lines {
                            for (line_num, line) in
                                assembly.lines_with_pos().into_iter().enumerate()
                            {
                                #[allow(clippy::single_match)]
                                match &line.t {
                                    assembly::Line::Command(command) => match command {
                                        Command::Branch(_, _) => {
                                            if selected_lines.len() == 1
                                                && selected_lines.contains(&line_num)
                                            {
                                                let useflag_line_num =
                                                    compiled.useflag_from_branch(line_num).unwrap();
                                                let useflag_line =
                                                    assembly.line_with_pos(useflag_line_num);
                                                text_attrs.underline.insert(
                                                    useflag_line.start..useflag_line.end,
                                                    purple_underline,
                                                );
                                                let flag_lines = compiled
                                                    .flag_setters_from_useflag(useflag_line_num)
                                                    .unwrap();
                                                let flag_lines = flag_lines
                                                    .into_iter()
                                                    .map(|flag_line| {
                                                        assembly.line_with_pos(flag_line)
                                                    })
                                                    .collect::<Vec<_>>();
                                                for flag_line in flag_lines {
                                                    text_attrs.underline.insert(
                                                        flag_line.start..flag_line.end,
                                                        purple_underline,
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    },
                                    assembly::Line::Meta(meta) => match meta {
                                        assembly::Meta::UseFlags => {
                                            if selected_lines.len() == 1
                                                && selected_lines.contains(&line_num)
                                            {
                                                let flag_lines = compiled
                                                    .flag_setters_from_useflag(line_num)
                                                    .unwrap();
                                                let flag_lines = flag_lines
                                                    .into_iter()
                                                    .map(|flag_line| {
                                                        assembly.line_with_pos(flag_line)
                                                    })
                                                    .collect::<Vec<_>>();
                                                for flag_line in flag_lines {
                                                    text_attrs.underline.insert(
                                                        flag_line.start..flag_line.end,
                                                        purple_underline,
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    },
                                }
                            }
                        }
                    }
                    Err(e) => match e {
                        assembly::CompileError::Invalid16BitValue { line } => {
                            let line = assembly.line_with_pos(*line);
                            match &line.t {
                                assembly::Line::Command(Command::Value(v)) => {
                                    text_attrs.underline.insert(v.start..v.end, red_underline);
                                }
                                _ => {
                                    text_attrs
                                        .underline
                                        .insert(line.start..line.end, red_underline);
                                }
                            }
                        }

                        assembly::CompileError::MissingLabel { line, .. } => {
                            let line = assembly.line_with_pos(*line);
                            match &line.t {
                                assembly::Line::Command(Command::Jump(label))
                                | assembly::Line::Command(Command::Branch(_, label))
                                | assembly::Line::Command(Command::Call(label)) => {
                                    text_attrs
                                        .underline
                                        .insert(label.start..label.end, red_underline);
                                }
                                _ => panic!(
                                    "Other lines should not panic here since they have no label argument."
                                ),
                            }
                        }

                        assembly::CompileError::JumpOrBranchToOtherPage { line } => {
                            let line = assembly.line_with_pos(*line);
                            match &line.t {
                                assembly::Line::Command(Command::Jump(label))
                                | assembly::Line::Command(Command::Branch(_, label)) => {
                                    text_attrs
                                        .underline
                                        .insert(label.start..label.end, red_underline);
                                }
                                _ => panic!(
                                    "Other lines should not panic here since they have no label argument."
                                ),
                            }
                        }
                        assembly::CompileError::BadUseflagsWithBranch {
                            branch_line,
                            useflags_line,
                        } => {
                            let branch_line = assembly.line_with_pos(*branch_line);
                            let useflags_line = assembly.line_with_pos(*useflags_line);
                            text_attrs
                                .underline
                                .insert(branch_line.start..branch_line.end, red_underline);
                            text_attrs
                                .underline
                                .insert(useflags_line.start..useflags_line.end, red_underline);
                        }
                        assembly::CompileError::BadUseflags { useflags_line } => {
                            let useflags_line = assembly.line_with_pos(*useflags_line);
                            text_attrs
                                .underline
                                .insert(useflags_line.start..useflags_line.end, red_underline);
                        }
                        assembly::CompileError::BranchWithoutUseflags { branch_line } => {
                            let branch_line = assembly.line_with_pos(*branch_line);
                            text_attrs
                                .underline
                                .insert(branch_line.start..branch_line.end, red_underline);
                        }
                        assembly::CompileError::PageFull { page } => {
                            for (start, end) in page_layout.get_page_text_intervals(page) {
                                text_attrs.underline.insert(start..end, red_underline);
                            }
                        }
                    },
                },
                Err(e) => match e {
                    assembly::LayoutPagesError::DuplicateLabel { line, .. } => {
                        let line = assembly.line_with_pos(*line);
                        text_attrs
                            .underline
                            .insert(line.start..line.end, red_underline);
                    }
                    assembly::LayoutPagesError::MissingPageStart { line } => {
                        let line = assembly.line_with_pos(*line);
                        text_attrs.underline.insert(0..line.end, red_underline);
                    }
                },
            }
        }
        Err(e) => match e {
            lalrpop_util::ParseError::InvalidToken { location } => {
                text_attrs
                    .underline
                    .insert(*location..*location + 1, red_underline);
            }
            lalrpop_util::ParseError::UnrecognizedEof { location, .. } => {
                text_attrs
                    .underline
                    .insert(location - 1..*location, red_underline);
            }
            lalrpop_util::ParseError::UnrecognizedToken { token, .. } => {
                text_attrs.underline.insert(token.0..token.2, red_underline);
            }
            lalrpop_util::ParseError::ExtraToken { token } => {
                text_attrs.underline.insert(token.0..token.2, red_underline);
            }
            lalrpop_util::ParseError::User { .. } => {
                text_attrs.underline.insert(0.., red_underline);
            }
        },
    }

    let mut job = LayoutJob::default();
    for i in 0..text.len() {
        job.append(
            &text[i..i + 1],
            0.0,
            TextFormat {
                color: *text_attrs.colour.get(i).unwrap_or(&visuals.text_color()),
                underline: *text_attrs.underline.get(i).unwrap_or(&Stroke::default()),
                italics: *text_attrs.italics.get(i).unwrap_or(&false),
                ..Default::default()
            },
        );
    }
    job
}

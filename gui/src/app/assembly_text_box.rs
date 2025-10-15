use crate::app::state::{CompileResult, State};
use assembly::{Command, CompileSuccess, Label, WithPos, load_assembly};
use btree_range_map::RangeMap;
use egui::{Color32, Stroke, TextBuffer, TextFormat, Visuals, text::LayoutJob};

pub fn update(
    state: &mut State,
    compile_result: &CompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    // Define layouter closure
    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
        let mut job = layout_job(text.as_str(), compile_result, ui.visuals());
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };

    ui.add(
        egui::TextEdit::multiline(&mut state.text)
            .font(egui::TextStyle::Monospace)
            .desired_rows(20)
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter),
    );
}

#[derive(Default, Debug)]
struct TextAttrs {
    colour: RangeMap<usize, Color32>,
    underline: RangeMap<usize, Stroke>,
    italics: RangeMap<usize, bool>,
}

/// Highlights all instances of "foo" in blue.
fn layout_job(text: &str, compile_result: &CompileResult, visuals: &Visuals) -> LayoutJob {
    let mut text_attrs = TextAttrs::default();

    let red_underline = Stroke {
        width: 1.0,
        color: Color32::RED,
    };

    match compile_result {
        Ok((compile_result, assembly)) => {
            // Parse success; apply colouring to text
            for WithPos {
                start,
                end,
                t: line,
            } in assembly.lines_with_pos()
            {
                text_attrs
                    .colour
                    .insert(*start..*end, visuals.strong_text_color());

                let add_label = |text_attrs: &mut TextAttrs, label: &WithPos<Label>| {
                    text_attrs.colour.insert(
                        label.start..label.end,
                        visuals.text_color().lerp_to_gamma(Color32::GREEN, 0.5),
                    );
                };

                let add_register =
                    |text_attrs: &mut TextAttrs, register: &WithPos<assembly::Nibble>| {
                        text_attrs.colour.insert(
                            register.start..register.end,
                            visuals.text_color().lerp_to_gamma(Color32::YELLOW, 0.5),
                        );
                    };

                match line {
                    assembly::Line::Command(command) => match command {
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

            // Compile errors
            match compile_result {
                Ok(_) => {}
                Err(e) => {
                    println!("{:?}", e);

                    match e {
                        assembly::CompileError::DuplicateLabel { line, label } => {
                            let line = assembly.line_with_pos(*line);
                            text_attrs
                                .underline
                                .insert(line.start..line.end, red_underline);
                        }
                        assembly::CompileError::MissingLabel { line } => {
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
                        assembly::CompileError::MissingPageStart { line } => {
                            let line = assembly.line_with_pos(*line);
                            text_attrs.underline.insert(0..line.end, red_underline);
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
                        assembly::CompileError::BadUseflags {
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
                    }
                }
            }
        }

        Err(e) => {
            // Parse errors
            println!("{}", e);
            match e {
                lalrpop_util::ParseError::InvalidToken { location } => {
                    text_attrs
                        .underline
                        .insert(*location..*location + 1, red_underline);
                }
                lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
                    text_attrs
                        .underline
                        .insert(location - 1..*location, red_underline);
                }
                lalrpop_util::ParseError::UnrecognizedToken { token, expected } => {
                    text_attrs.underline.insert(token.0..token.2, red_underline);
                }
                lalrpop_util::ParseError::ExtraToken { token } => {
                    text_attrs.underline.insert(token.0..token.2, red_underline);
                }
                lalrpop_util::ParseError::User { error } => {
                    text_attrs.underline.insert(0.., red_underline);
                }
            }
        }
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

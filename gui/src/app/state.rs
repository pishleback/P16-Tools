use assembly::{Assembly, CompileError, CompileSuccess, compile_assembly, load_assembly};
use egui::Ui;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct State {
    pub text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            text: "PASS".into(),
        }
    }
}

pub type CompileResult<'a> = Result<
    (Result<CompileSuccess, CompileError>, Assembly),
    lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'a>, &'static str>,
>;

impl State {
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let source = self.text.clone();
        let compile_result: CompileResult =
            load_assembly(&source).map(|assembly| (compile_assembly(&assembly), assembly));

        // Left panel with buttons
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Test Buttons");
                if ui.button("Clear").clicked() {
                    self.text.clear();
                }
            });

        // Central text area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Assembly").show(ui, |ui| {
                        super::assembly_text_box::update(self, &compile_result, ctx, frame, ui);
                    });

                    egui::CollapsingHeader::new("Simulator").show(ui, |ui| {
                        ui.columns(3, |ui| {
                            ui[0].label("Column 111111");
                            ui[1].label("Column 2");
                            ui[2].label("Column 3");
                        });

                        ui.label("TODO: Simulator");
                    });
                });
        });
    }
}

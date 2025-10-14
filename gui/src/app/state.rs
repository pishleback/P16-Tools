use egui::{Color32, TextBuffer, TextFormat, text::LayoutJob};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct State {
    text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            text: "foo bar".into(),
        }
    }
}

impl State {
    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Left panel with buttons
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Demo Buttons");
                if ui.button("Button 1").clicked() {
                    self.text = "Button 1 clicked!".to_string();
                }
                if ui.button("Button 2").clicked() {
                    self.text = "Button 2 clicked!".to_string();
                }
                if ui.button("Clear").clicked() {
                    self.text.clear();
                }
            });

        // Central text area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Central Text Area");
            ui.add_space(10.0);

            // Define layouter closure
            let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
                let mut job = highlight_foo(text.as_str());
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };

            ui.add(
                egui::TextEdit::multiline(&mut self.text)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(20)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            )
        });
    }
}

/// Highlights all instances of "foo" in blue.
fn highlight_foo(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let normal = TextFormat {
        color: Color32::WHITE,
        ..Default::default()
    };
    let highlight = TextFormat {
        color: Color32::from_rgb(100, 180, 255),
        ..Default::default()
    };

    let mut start = 0;
    for (idx, _) in text.match_indices("foo") {
        // Add preceding normal text
        if start < idx {
            job.append(&text[start..idx], 0.0, normal.clone());
        }
        // Add highlighted “foo”
        job.append("foo", 0.0, highlight.clone());
        start = idx + 3;
    }
    // Add remaining text
    if start < text.len() {
        job.append(&text[start..], 0.0, normal);
    }

    job
}

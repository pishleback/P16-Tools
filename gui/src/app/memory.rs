use assembly::{FullCompileResult, Nibble, RAM_SIZE};

pub fn update(
    compile_result: &FullCompileResult,
    _ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
) {
    if let Ok((Ok((Ok(compiled), page_layout)), _assembly)) = &compile_result {
        let raw_memory = compiled.memory().clone();

        egui::CollapsingHeader::new("Memory").show(ui, |ui| {
            // Show ROM pages
            for rom_page in (0..16).map(|n| Nibble::new(n).unwrap()) {
                let mut nibbles = raw_memory
                    .rom_page(rom_page)
                    .nibbles()
                    .iter()
                    .map(|n| n.hex_str())
                    .collect::<String>();
                nibbles = String::from(nibbles.trim_end_matches('0'));
                if !nibbles.is_empty() {
                    egui::CollapsingHeader::new(format!("ROM {}", rom_page.hex_str())).show(
                        ui,
                        |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut nibbles)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(1)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY),
                            );
                        },
                    );
                }
            }

            // Show RAM
            let mut ram_data = (0..RAM_SIZE as usize)
                .map(|i| raw_memory.ram().data()[i])
                .collect::<Vec<_>>();
            while ram_data.last().map(|v| *v == 0).unwrap_or(false) {
                ram_data.pop();
            }
            if !ram_data.is_empty() {
                let mut z = ram_data
                    .into_iter()
                    .map(|v| format!("{v}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                egui::CollapsingHeader::new("RAM").show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut z)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(1)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY),
                    );
                });
            }
        });
    }
}

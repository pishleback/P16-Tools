use assembly::{InputQueue, OctDigit};
use std::sync::{Arc, Mutex};

pub struct State {
    externals: Vec<Box<dyn External>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            externals: vec![
                Box::new(DisplayV1::new(vec![OctDigit::O4])),
                Box::new(MultiplierV1::new(vec![OctDigit::O5], vec![OctDigit::O6])),
            ],
        }
    }
}

impl State {
    pub fn handle_io(
        &mut self,
        input_queue: Arc<Mutex<InputQueue>>,
        outputs: Vec<(Vec<OctDigit>, u16)>,
    ) {
        for external in &mut self.externals {
            external.handle_io(input_queue.clone(), &outputs);
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        for external in &mut self.externals {
            external.update(ctx, frame);
        }
    }
}

pub trait External {
    fn handle_io(&mut self, input_queue: Arc<Mutex<InputQueue>>, outputs: &[(Vec<OctDigit>, u16)]);

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame);
}

pub struct DisplayV1 {
    x: u16,
    fill_x: bool,
    y: u16,
    fill_y: bool,
    pixels: [[bool; 64]; 64],
    path: Vec<OctDigit>,
}

impl DisplayV1 {
    pub fn new(path: Vec<OctDigit>) -> Self {
        DisplayV1 {
            x: 0,
            fill_x: false,
            y: 0,
            fill_y: false,
            path,
            pixels: std::array::from_fn(|_x| std::array::from_fn(|_y| false)),
        }
    }
}

impl External for DisplayV1 {
    fn handle_io(
        &mut self,
        _input_queue: Arc<Mutex<InputQueue>>,
        outputs: &[(Vec<OctDigit>, u16)],
    ) {
        #[derive(PartialEq, Eq)]
        enum Coord {
            X,
            Y,
        }
        for (path, value) in outputs {
            let (first, last) = path.split_at(self.path.len());
            if first == self.path && last.len() == 1 {
                // Are we setting x or y
                let coord = if (last[0].as_u8() & 1) == 0 {
                    Coord::X
                } else {
                    Coord::Y
                };
                // Should we set all of this coordinate or just the one given?
                let fill = (last[0].as_u8() & 2) != 0;
                // Should we set it to on or off (only applicable when setting Y)
                let state = (last[0].as_u8() & 4) != 0;

                match coord {
                    Coord::X => {
                        self.x = (*value) % 64;
                        self.fill_x = fill;
                    }
                    Coord::Y => {
                        self.y = (*value) % 64;
                        self.fill_y = fill;

                        let xs = if self.fill_x {
                            (0..64).collect()
                        } else {
                            vec![self.x]
                        };
                        let ys = if self.fill_y {
                            (0..64).collect()
                        } else {
                            vec![self.y]
                        };

                        for x in xs {
                            for y in &ys {
                                self.pixels[x as usize][*y as usize] = state;
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Display").show(ctx, |ui| {
            // Define the size of each pixel
            let pixel_size = 10.0; // adjust if needed

            // Make a fixed-size area for the grid
            let grid_size = pixel_size * 64.0;
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(grid_size, grid_size),
                egui::Sense::hover(), // allows hover/click if needed
            );

            // Get a painter for this rect
            let painter = ui.painter_at(rect);

            for y in 0..64 {
                for x in 0..64 {
                    // Determine if this pixel is "on" or "off"
                    let is_on = self.pixels[x][63 - y];

                    // Set color based on state
                    let color = if is_on {
                        egui::Color32::from_rgb(243, 227, 107)
                    } else {
                        egui::Color32::from_rgb(107, 75, 58)
                    };

                    // Compute the pixel rectangle
                    let pixel_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(x as f32 * pixel_size, y as f32 * pixel_size),
                        egui::vec2(pixel_size, pixel_size),
                    );

                    // Draw the rectangle
                    painter.rect_filled(pixel_rect, 0.0, color);
                }
            }
        });
    }
}

pub struct MultiplierV1 {
    reg_a: u16,
    path_a: Vec<OctDigit>,
    path_b: Vec<OctDigit>,
}

impl MultiplierV1 {
    pub fn new(path_a: Vec<OctDigit>, path_b: Vec<OctDigit>) -> Self {
        debug_assert_ne!(path_a, path_b);
        MultiplierV1 {
            reg_a: 0,
            path_a,
            path_b,
        }
    }
}

impl External for MultiplierV1 {
    fn handle_io(
        &mut self,
        input_queue: Arc<Mutex<InputQueue>>,
        outputs: &[(Vec<assembly::OctDigit>, u16)],
    ) {
        for (path, value) in outputs {
            if path == &self.path_a {
                self.reg_a = *value;
            }
            if path == &self.path_b {
                let s = (self.reg_a as u32) * (*value as u32);
                let (s_low, s_high) = (s as u16, (s >> 16) as u16);
                let mut inputs = input_queue.lock().unwrap();
                inputs.push(s_low);
                inputs.push(s_high);
            }
        }
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}

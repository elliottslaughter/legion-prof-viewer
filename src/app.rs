use egui::{
    Align2, Color32, Mesh, NumExt, Pos2, Rect, ScrollArea, Sense, Shape, Stroke, TextStyle, Vec2,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Timestamp(pub u64 /* ns */);

#[derive(Default, Deserialize, Serialize)]
pub struct Slot {
    expanded: bool,
    short_name: String,
    long_name: String,
    max_rows: u64,
}

const UNEXPANDED_ROWS: u64 = 4;

impl Slot {
    fn ui(&mut self, ui: &mut egui::Ui, rect: Rect, row_height: f32) {
        let response = ui.allocate_rect(rect, egui::Sense::click());

        let font_id = TextStyle::Body.resolve(ui.style());
        let visuals = ui.style().interact_selectable(&response, false);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
        ui.painter().text(
            rect.min + Vec2::new(4.0, 4.0),
            Align2::LEFT_TOP,
            &self.short_name,
            font_id.clone(),
            visuals.text_color(),
        );

        // This will take effect next frame because we can't redraw this widget now
        if response.clicked() {
            self.expanded = !self.expanded;
        }
    }

    fn rows(&self) -> u64 {
        if self.expanded {
            self.max_rows
        } else {
            UNEXPANDED_ROWS
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Window {
    slots: Vec<Slot>,
    min_time: Timestamp,
    max_time: Timestamp,
    window_start: Timestamp,
    window_stop: Timestamp,
}

impl Window {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Use body font to figure out how tall to draw rectangles.
        let font_id = TextStyle::Body.resolve(ui.style());
        let row_height = ui.fonts().row_height(&font_id);

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_viewport(ui, |ui, viewport| {
                // First pass to figure out how many rows we have in total
                let mut total_rows = 0;
                for slot in &self.slots {
                    total_rows += slot.rows();
                }

                ui.set_height(row_height * total_rows as f32);
                ui.set_width(ui.available_width());

                let first_row = (viewport.min.y / row_height).floor().at_least(0.0) as u64;
                let last_row = (viewport.max.y / row_height).ceil() as u64 + 1;
                let last_row = last_row.at_most(total_rows);

                // Second pass to draw those that intersect with the window
                let mut row = 0;
                for slot in &mut self.slots {
                    let start_row = row;
                    row += slot.rows();

                    // Cull out-of-view slots
                    if row < first_row {
                        continue;
                    } else if start_row > last_row {
                        break;
                    }

                    let y_min = ui.min_rect().top() + start_row as f32 * row_height;
                    let y_max = ui.min_rect().top() + row as f32 * row_height;

                    let rect = ui
                        .min_rect()
                        .intersect(Rect::everything_below(y_min))
                        .intersect(Rect::everything_above(y_max));

                    slot.ui(ui, rect, row_height);
                }
            });
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)] // deserialize missing fields as default value
pub struct ProfViewer {
    window: Window,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    last_update: Instant,
}

impl Default for ProfViewer {
    fn default() -> Self {
        Self {
            window: Window::default(),
            #[cfg(not(target_arch = "wasm32"))]
            last_update: Instant::now(),
        }
    }
}

impl ProfViewer {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut result: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let mut rng = rand::thread_rng();
        const N: i32 = 16;
        result.window.slots.clear();
        for i in 0..N {
            let rows: u64 = rng.gen_range(0..64);
            result.window.slots.push(Slot {
                expanded: false,
                short_name: format!("s{}", i),
                long_name: format!("slot {}", i),
                max_rows: rows,
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            result.last_update = Instant::now();
        }

        result
    }
}

impl eframe::App for ProfViewer {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            window,
            #[cfg(not(target_arch = "wasm32"))]
            last_update,
        } = self;

        let mut _fps = 0.0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = Instant::now();
            _fps = 1.0 / now.duration_since(*last_update).as_secs_f64();
            *last_update = now;
        }

        #[cfg(not(target_arch = "wasm32"))]
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.label(format!("FPS: {:.0}", _fps));
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to(
                        "eframe",
                        "https://github.com/emilk/egui/tree/master/crates/eframe",
                    );
                    ui.label(".");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Test Heading");
            window.ui(ui);
            egui::warn_if_debug_build(ui);
        });
    }
}

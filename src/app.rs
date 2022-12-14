use egui::{
    Align2, Color32, Mesh, NumExt, Pos2, Rect, ScrollArea, Sense, Shape, Stroke, TextStyle, Vec2,
};
use egui_extras::{Column, TableBuilder};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Timestamp(pub u64 /* ns */);

pub struct Item {
    row: u64,
    start: f32,
    stop: f32,
}

#[derive(Deserialize, Serialize)]
pub struct Slot {
    expanded: bool,
    short_name: String,
    long_name: String,
    max_rows: u64,

    #[serde(skip)]
    items: Vec<Item>,
}

impl Slot {
    const UNEXPANDED_ROWS: u64 = 4;

    fn label(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());

        let style = ui.style();
        let font_id = TextStyle::Body.resolve(style);
        let visuals = style.interact_selectable(&response, false);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
        ui.painter().text(
            rect.min + style.spacing.item_spacing,
            Align2::LEFT_TOP,
            &self.short_name,
            font_id.clone(),
            visuals.text_color(),
        );

        // This will take effect next frame because we can't redraw this widget now
        // FIXME: this creates inconsistency because this updates before the viewer widget
        if response.clicked() {
            self.expanded = !self.expanded;
        }
    }

    fn viewer(&mut self, ui: &mut egui::Ui, row_height: f32) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());

        let style = ui.style();
        let visuals = style.interact_selectable(&response, false);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
        if self.expanded {
            let rows = self.rows();
            let mut i = 0;
            for item in &self.items {
                let min = rect.lerp(Vec2::new(
                    item.start,
                    (item.row as f32 + 0.05) / rows as f32,
                ));
                let max = rect.lerp(Vec2::new(item.stop, (item.row as f32 + 0.95) / rows as f32));
                let color = match i % 6 {
                    0 => Color32::BLUE,
                    1 => Color32::RED,
                    2 => Color32::GREEN,
                    3 => Color32::YELLOW,
                    4 => Color32::BROWN,
                    5 => Color32::LIGHT_GREEN,
                    _ => Color32::WHITE,
                };
                i += 1;
                ui.painter().rect(
                    Rect::from_min_max(min, max),
                    0.0,
                    color,
                    Stroke::NONE,
                );
            }
        }
    }

    fn rows(&self) -> u64 {
        if self.expanded {
            self.max_rows
        } else {
            Self::UNEXPANDED_ROWS
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

        let table = TableBuilder::new(ui)
            .auto_shrink([false; 2])
            .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
            .column(Column::exact(100.0))
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::MAX);

        table.body(|body| {
            body.heterogeneous_rows(
                self.slots
                    .iter()
                    .map(|slot| slot.rows() as f32 * row_height)
                    .collect::<Vec<_>>()
                    .into_iter(),
                |index, mut row| {
                    let slot = &mut self.slots[index];
                    row.col(|ui| slot.label(ui));
                    row.col(|ui| slot.viewer(ui, row_height));
                },
            )
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
            let mut items = Vec::new();
            for row in 0..rows {
                const M: u64 = 1000;
                for i in 0..M {
                    let start = (i as f32 + 0.05) / (M as f32);
                    let stop = (i as f32 + 0.95) / (M as f32);
                    items.push(Item { row, start, stop });
                }
            }
            result.window.slots.push(Slot {
                expanded: false,
                short_name: format!("s{}", i),
                long_name: format!("slot {}", i),
                max_rows: rows,
                items: items,
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

use egui::{Pos2, Rect, Vec2, Mesh, Shape, Color32, Stroke, TextStyle, ScrollArea, Sense, NumExt, Align2};
use rand::Rng;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Slot {
    expanded: bool,
    short_name: String,
    long_name: String,
    max_rows: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(default)] // deserialize missing fields as default value
pub struct ProfViewer {
    slots: Vec<Slot>,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    last_update: Instant,
}

impl Default for ProfViewer {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
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
        result.slots.clear();
        for i in 0..N {
            let rows: u64 = rng.gen_range(0..64);
            result.slots.push(Slot {
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
            slots,
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
            view_area(ui, slots);
            egui::warn_if_debug_build(ui);
        });
    }
}

pub fn view_area(ui: &mut egui::Ui, slots: &Vec<Slot>) {
    // Use body font to figure out how tall to draw rectangles.
    let font_id = TextStyle::Body.resolve(ui.style());
    let row_height = ui.fonts().row_height(&font_id);

    const UNEXPANDED_ROWS: u64 = 4;

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_viewport(ui, |ui, viewport| {
            // First pass to figure out how many rows we have in total
            let mut total_rows = 0;
            for slot in slots {
                if slot.expanded {
                    total_rows += slot.max_rows;
                } else {
                    total_rows += UNEXPANDED_ROWS;
                }
            }

            ui.set_height(row_height * total_rows as f32);
            ui.set_width(ui.available_width());

            let first_row = (viewport.min.y / row_height).floor().at_least(0.0) as u64;
            let last_row = (viewport.max.y / row_height).ceil() as u64 + 1;
            let last_row = last_row.at_most(total_rows);

            // let visuals = ui.style().interact_selectable(&response, false);
            let visuals = ui.style().noninteractive(); // Hack, need to figure out how to make this responsive
            let font_id = TextStyle::Body.resolve(ui.style());

            // Second pass to draw those that intersect with the window
            let mut row = 0;
            let mut used_rect = Rect::NOTHING;
            for slot in slots {
                let start_row = row;
                if slot.expanded {
                    row += slot.max_rows;
                } else {
                    row += UNEXPANDED_ROWS;
                }

                // Prune slots out of window
                if start_row > last_row || row < first_row {
                    continue;
                }

                let y_min = ui.min_rect().top() + start_row as f32 * row_height;
                let y_max = ui.min_rect().top() + row as f32 * row_height;

                let slot_rect = ui.min_rect().intersect(Rect::everything_below(y_min)).intersect(Rect::everything_above(y_max));

                ui.painter().rect(
                    slot_rect,
                    0.0,
                    Color32::GREEN/*visuals.bg_fill*/, visuals.bg_stroke,
                    // Color32::GREEN, Color32::BLACK
                    //visuals.bg_fill, visuals.bg_stroke
                );
                ui.painter().text(
                    slot_rect.min,
                    Align2::LEFT_TOP,
                    slot.short_name.clone(),
                    font_id.clone(),
                    visuals.text_color(),
                );
                used_rect = used_rect.union(slot_rect);
            }

            ui.allocate_rect(used_rect, Sense::hover());
        });
}

// pub fn slot_ui(ui: &mut egui::Ui, slots: &Slot) -> egui::Response {
//     let (rect, mut response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
//     let hover = response.hover_pos(); // where is the mouse hovering (if any)
//     if response.clicked() {
//         // This is the click handler
//     }
//     if ui.is_rect_visible(rect) {
//         // Draw the widget
//         let visuals = ui.style().interact_selectable(&response, false);
//         // let rect = rect.expand(visuals.expansion);
//         for r in rects {
//             let r2 = Rect::from_min_max(
//                 rect.lerp(r.r.left_top().to_vec2()),
//                 rect.lerp(r.r.right_bottom().to_vec2()),
//             );
//             if let Some(h) = hover {
//                 if r2.contains(h) {
//                     let r2 = r2.expand(visuals.expansion);
//                     ui.painter()
//                         .rect(r2, 0.0, r.c, /*visuals.bg_fill,*/ visuals.bg_stroke);
//                     continue;
//                 }
//             };
//             // let r2 = r2.expand(visuals.expansion);
//             ui.painter()
//                 .rect(r2, 0.0, r.c, Stroke::NONE);
//         }
//     }
//     response
// }

// pub fn slot(rects: &Slot) -> impl egui::Widget + '_ {
//     move |ui: &mut egui::Ui| slot_ui(ui, rects)
// }

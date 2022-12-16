use egui::{Align2, Color32, NumExt, Pos2, Rect, ScrollArea, Stroke, TextStyle, Vec2};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Timestamp(pub u64 /* ns */);

/// Overview:
///   Window -> Panel
///   Panel -> Summary, { Panel | Slot } *
///   Summary
///   Slot -> Item *
///
/// Window:
///   * Owns the ScrollArea (there is only **ONE** ScrollArea)
///   * Handles pan/zoom (there is only **ONE** pan/zoom setting)
///
/// Panel:
///   * Table widget for (nested) cells
///   * Each row contains: label, content
///
/// Summary
///   * Utilization widget
///
/// Slot:
///   * Viewer widget for items

// DO NOT derive (de)serialize, we will never serialize this
pub struct Item {
    _row: u64,
    start: f32,
    stop: f32,
}

#[derive(Default)]
pub struct Summary {
    utilization: Vec<f32>,
}

#[derive(Default)]
pub struct Slot {
    expanded: bool,
    short_name: String,
    long_name: String,
    max_rows: u64,
    items: Vec<Vec<Item>>, // row -> [item]
}

#[derive(Default)]
pub struct Panel<S: Entry> {
    expanded: bool,
    short_name: String,
    long_name: String,

    summary: Option<Summary>,
    slots: Vec<S>,
}

pub trait Entry {
    fn label_text(&self) -> &str;
    fn hover_text(&self) -> &str;

    fn label(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let response = ui.allocate_rect(
            rect,
            if self.is_expandable() {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );

        let style = ui.style();
        let font_id = TextStyle::Body.resolve(style);
        let visuals = if self.is_expandable() {
            style.interact_selectable(&response, false)
        } else {
            *style.noninteractive()
        };

        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
        ui.painter().text(
            rect.min + style.spacing.item_spacing,
            Align2::LEFT_TOP,
            self.label_text(),
            font_id,
            visuals.text_color(),
        );

        if response.clicked() {
            // This will take effect next frame because we can't redraw this widget now
            self.toggle_expanded();
        } else if response.hovered() {
            response.on_hover_text(self.hover_text());
        }
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: Rect, viewport: Rect, row_height: f32);

    fn height(&self, row_height: f32) -> f32;

    fn is_expandable(&self) -> bool;

    fn toggle_expanded(&mut self);
}

impl Entry for Summary {
    fn label_text(&self) -> &str {
        "Summary"
    }
    fn hover_text(&self) -> &str {
        "Utilization Plot of Processor/Channel/Memory Usage"
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: Rect, _viewport: Rect, _row_height: f32) {
        let response = ui.allocate_rect(rect, egui::Sense::hover());

        let style = ui.style();
        let visuals = style.interact_selectable(&response, false);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
    }

    fn height(&self, row_height: f32) -> f32 {
        const ROWS: u64 = 4;
        ROWS as f32 * row_height
    }

    fn is_expandable(&self) -> bool {
        false
    }

    fn toggle_expanded(&mut self) {
        unreachable!();
    }
}

impl Slot {
    fn rows(&self) -> u64 {
        const UNEXPANDED_ROWS: u64 = 2;
        if self.expanded {
            self.max_rows.at_least(UNEXPANDED_ROWS)
        } else {
            UNEXPANDED_ROWS
        }
    }

    fn generate(&mut self) {
        let mut items = Vec::new();
        for row in 0..self.max_rows {
            let mut row_items = Vec::new();
            const N: u64 = 1000;
            for i in 0..N {
                let start = (i as f32 + 0.05) / (N as f32);
                let stop = (i as f32 + 0.95) / (N as f32);
                row_items.push(Item {
                    _row: row,
                    start,
                    stop,
                });
            }
            items.push(row_items);
        }
        self.items = items;
    }
}

impl Entry for Slot {
    fn label_text(&self) -> &str {
        &self.short_name
    }
    fn hover_text(&self) -> &str {
        &self.long_name
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: Rect, viewport: Rect, _row_height: f32) {
        let response = ui.allocate_rect(rect, egui::Sense::hover());
        let mut hover_pos = response.hover_pos(); // where is the mouse hovering?

        if self.expanded {
            if self.items.is_empty() {
                self.generate();
            }

            let style = ui.style();
            let visuals = style.interact_selectable(&response, false);
            ui.painter()
                .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

            let rows = self.rows();
            let mut i = 0;
            for (row, row_items) in self.items.iter().enumerate() {
                // Need to reverse the rows because we're working in screen space
                let irow = self.items.len() - row - 1;

                // We want to do this first on rows, so that we can cut the
                // entire row if we don't need it

                // Compute bounds for the whole row
                let row_min = rect.lerp(Vec2::new(0.0, (irow as f32 + 0.05) / rows as f32));
                let row_max = rect.lerp(Vec2::new(1.0, (irow as f32 + 0.95) / rows as f32));

                // Cull if out of bounds
                // Note: need to shift by rect.min to get to viewport space
                if row_max.y - rect.min.y < viewport.min.y {
                    break;
                } else if row_min.y - rect.min.y > viewport.max.y {
                    continue;
                }

                // Check if mouse is hovering over this row
                let row_rect = Rect::from_min_max(row_min, row_max);
                let row_hover = hover_pos.map_or(false, |h| row_rect.contains(h));

                // Now handle the items
                for item in row_items {
                    let min = rect.lerp(Vec2::new(item.start, (irow as f32 + 0.05) / rows as f32));
                    let max = rect.lerp(Vec2::new(item.stop, (irow as f32 + 0.95) / rows as f32));
                    let color = match i % 7 {
                        0 => Color32::BLUE,
                        1 => Color32::RED,
                        2 => Color32::GREEN,
                        3 => Color32::YELLOW,
                        4 => Color32::KHAKI,
                        5 => Color32::DARK_GREEN,
                        6 => Color32::DARK_BLUE,
                        _ => Color32::WHITE,
                    };
                    i += 1;
                    let item_rect = Rect::from_min_max(min, max);
                    if row_hover && hover_pos.map_or(false, |h| item_rect.contains(h)) {
                        hover_pos = None;

                        // Hack: create a new response for this rect specifically
                        // so we can use the hover methods...
                        // (Elliott: I assume this is more efficient than allocating
                        // every single rect.)
                        let item_response = ui.allocate_rect(item_rect, egui::Sense::hover());
                        item_response.on_hover_text(format!(
                            "Item: {} {} Row: {}",
                            item.start, item.stop, row
                        ));
                    }
                    ui.painter().rect(item_rect, 0.0, color, Stroke::NONE);
                }
            }
        }
    }

    fn height(&self, row_height: f32) -> f32 {
        self.rows() as f32 * row_height
    }

    fn is_expandable(&self) -> bool {
        true
    }

    fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
}

impl<S: Entry> Panel<S> {
    fn render<T: Entry>(
        ui: &mut egui::Ui,
        rect: Rect,
        viewport: Rect,
        slot: &mut T,
        y: &mut f32,
        row_height: f32,
    ) -> bool {
        const LABEL_WIDTH: f32 = 80.0;
        const COL_PADDING: f32 = 4.0;
        const ROW_PADDING: f32 = 4.0;

        // Compute the size of this slot
        // This is in screen (i.e., rect) space
        let min_y = *y;
        let max_y = min_y + slot.height(row_height);
        *y = max_y + ROW_PADDING;

        // Cull if out of bounds
        // Note: need to shift by rect.min to get to viewport space
        if max_y - rect.min.y < viewport.min.y {
            return false;
        } else if min_y - rect.min.y > viewport.max.y {
            return true;
        }

        // Draw label and content
        let label_min = rect.min.x;
        let label_max = (rect.min.x + LABEL_WIDTH).at_most(rect.max.x);
        let content_min = (label_max + COL_PADDING).at_most(rect.max.x);
        let content_max = rect.max.x;

        let label_subrect =
            Rect::from_min_max(Pos2::new(label_min, min_y), Pos2::new(label_max, max_y));
        let content_subrect =
            Rect::from_min_max(Pos2::new(content_min, min_y), Pos2::new(content_max, max_y));

        // Shift viewport up by the amount consumed
        // Invariant: (0, 0) in viewport is rect.min
        //   (i.e., subtracting rect.min gets us from screen space to viewport space)
        // Note: viewport.min is NOT necessarily (0, 0)
        let content_viewport = viewport.translate(Vec2::new(0.0, rect.min.y - min_y));

        slot.content(ui, content_subrect, content_viewport, row_height);
        slot.label(ui, label_subrect);

        false
    }
}

impl<S: Entry> Entry for Panel<S> {
    fn label_text(&self) -> &str {
        &self.short_name
    }
    fn hover_text(&self) -> &str {
        &self.long_name
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: Rect, viewport: Rect, row_height: f32) {
        let mut y = rect.min.y;
        if let Some(summary) = &mut self.summary {
            Self::render(ui, rect, viewport, summary, &mut y, row_height);
        }

        if self.expanded {
            for slot in &mut self.slots {
                if Self::render(ui, rect, viewport, slot, &mut y, row_height) {
                    break;
                }
            }
        }
    }

    fn height(&self, row_height: f32) -> f32 {
        const UNEXPANDED_ROWS: u64 = 4;
        const ROW_PADDING: f32 = 4.0;

        let mut total = 0.0;
        let mut rows: i64 = 0;
        if let Some(summary) = &self.summary {
            total += summary.height(row_height);
            rows += 1;
        } else if !self.expanded {
            // Need some minimum space if this panel has no summary and is collapsed
            total += UNEXPANDED_ROWS as f32 * row_height;
            rows += 1;
        }

        if self.expanded {
            for slot in &self.slots {
                total += slot.height(row_height);
            }
            rows += self.slots.len() as i64;
        }

        total += (rows - 1).at_least(0) as f32 * ROW_PADDING;

        total
    }

    fn is_expandable(&self) -> bool {
        true
    }

    fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Window {
    #[serde(skip)]
    panel: Panel<Panel<Panel<Slot>>>, // nodes -> kind -> proc/chan/mem
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
                let height = self.panel.height(row_height);
                ui.set_height(height);
                ui.set_width(ui.available_width());

                let rect = Rect::from_min_size(ui.min_rect().min, viewport.size());

                // Root panel has no label
                self.panel.content(ui, rect, viewport, row_height);
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
        const NODES: i32 = 8192;
        const PROCS: i32 = 8;
        let mut node_slots = Vec::new();
        for node in 0..NODES {
            let mut kind_slots = Vec::new();
            for kind in &["CPU", "GPU", "Util", "Chan"] {
                let mut proc_slots = Vec::new();
                for proc in 0..PROCS {
                    let rows: u64 = rng.gen_range(0..64);
                    let items = Vec::new();
                    // Leave items empty, we'll generate it later
                    proc_slots.push(Slot {
                        expanded: true,
                        short_name: format!("{}{}", kind.chars().next().unwrap().to_lowercase(), proc),
                        long_name: format!("Node {} {} {}", node, kind, proc),
                        max_rows: rows,
                        items,
                    });
                }
                kind_slots.push(Panel {
                    expanded: false,
                    short_name: kind.to_lowercase(),
                    long_name: format!("Node {} {}", node, kind),
                    summary: Some(Summary::default()),
                    slots: proc_slots,
                });
            }
            node_slots.push(Panel {
                expanded: true,
                short_name: format!("n{}", node),
                long_name: format!("Node {}", node),
                summary: None,
                slots: kind_slots,
            });
        }
        result.window.panel = Panel {
            expanded: true,
            short_name: "root".to_owned(),
            long_name: "root".to_owned(),
            summary: None,
            slots: node_slots,
        };

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
            ui.heading("Legion Prof");
            ui.label("Welcome to the Legion Prof tech demo. This is NOT a working profiler, because the data here is completely fake. However, this is a testbed for designing features for a potential future profiler interface. Feel free to click around and check it out.");

            ui.separator();

            ui.label("Things you can do in this version:");

            ui.label(" 1. Click a node to collapse it.");
            ui.label(" 2. Click a processor/channel kind to expand it.");
            ui.label(" 3. Click a processor or channel to collapse it.");
            ui.label(" 4. Hover over a task to see a tooltip.");

            ui.separator();

            ui.label("Things that do NOT work yet:");

            ui.label(" 1. Pan/zoom.");
            ui.label(" 2. Expand all of a kind.");
            ui.label(" 3. Search.");
            ui.label(" 4. Relationships (e.g., dependencies).");

            ui.separator();

            ui.label("The app should stay responsive throughout.");

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

                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    ui.label(format!("FPS: {:.0}", _fps));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            window.ui(ui);
            egui::warn_if_debug_build(ui);
        });
    }
}

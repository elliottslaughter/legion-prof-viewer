use egui::{Align2, Color32, NumExt, Pos2, Rect, ScrollArea, Slider, Stroke, TextStyle, Vec2};
use rand::Rng;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
struct Timestamp(u64 /* ns */);

/// Overview:
///   ProfApp -> Context, Window *
///   Window -> Config, Panel
///   Panel -> Summary, { Panel | Slot } *
///   Summary
///   Slot -> Item *
///
/// Context:
///   * Global configuration state (i.e., for all profiles)
///
/// Window:
///   * One Windows per profile
///   * Owns the ScrollArea (there is only **ONE** ScrollArea)
///   * Handles pan/zoom (there is only **ONE** pan/zoom setting)
///
/// Config:
///   * Window configuration state (i.e., specific to a profile)
///
/// Panel:
///   * One Panel for each level of nesting in the profile (root, node, kind)
///   * Table widget for (nested) cells
///   * Each row contains: label, content
///
/// Summary:
///   * Utilization widget
///
/// Slot:
///   * One Slot for each processor, channel, memory
///   * Viewer widget for items

// DO NOT derive (de)serialize, we will never serialize this
struct Item {
    _row: u64,
    start: f32,
    stop: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
struct UtilPoint {
    time: f32,
    util: f32,
}

#[derive(Default)]
struct Summary {
    utilization: Vec<UtilPoint>,
    color: Color32,
}

#[derive(Default)]
struct Slot {
    expanded: bool,
    short_name: String,
    long_name: String,
    max_rows: u64,
    items: Vec<Vec<Item>>, // row -> [item]
}

#[derive(Default)]
struct Panel<S: Entry> {
    expanded: bool,
    short_name: String,
    long_name: String,
    level: u64,

    summary: Option<Summary>,
    slots: Vec<S>,
}

#[derive(Default, Deserialize, Serialize)]
struct Config {
    // Node selection controls
    min_node: u64,
    max_node: u64,
}

#[derive(Default, Deserialize, Serialize)]
struct Window {
    #[serde(skip)]
    panel: Panel<Panel<Panel<Slot>>>, // nodes -> kind -> proc/chan/mem
    min_time: Timestamp,
    max_time: Timestamp,
    index: u64,
    kinds: Vec<String>,
    config: Config,
}

#[derive(Default, Deserialize, Serialize)]
struct Context {
    row_height: f32,

    // Visible time range
    window_start: Timestamp,
    window_stop: Timestamp,

    #[serde(skip)]
    rng: rand::rngs::ThreadRng,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)] // deserialize missing fields as default value
pub struct ProfApp {
    #[serde(skip)]
    windows: Vec<Window>,

    cx: Context,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    last_update: Option<Instant>,
}

trait Entry {
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

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        viewport: Rect,
        config: &mut Config,
        cx: &mut Context,
    );

    fn height(&self, config: &Config, cx: &Context) -> f32;

    fn is_expandable(&self) -> bool;

    fn toggle_expanded(&mut self);
}

impl Summary {
    fn generate_point(
        &mut self,
        first: UtilPoint,
        last: UtilPoint,
        level: i32,
        max_level: i32,
        cx: &mut Context,
    ) {
        let time = (first.time + last.time) * 0.5;
        let util = (first.util + last.util) * 0.5;
        let diff = (cx.rng.gen::<f32>() - 0.5) / 1.2_f32.powi(max_level - level);
        let util = (util + diff).at_least(0.0).at_most(1.0);
        let point = UtilPoint { time, util };
        if level > 0 {
            self.generate_point(first, point, level - 1, max_level, cx);
        }
        self.utilization.push(point);
        if level > 0 {
            self.generate_point(point, last, level - 1, max_level, cx);
        }
    }

    fn generate(&mut self, cx: &mut Context) {
        const LEVELS: i32 = 8;
        let first = UtilPoint {
            time: 0.0,
            util: cx.rng.gen(),
        };
        let last = UtilPoint {
            time: 1.0,
            util: cx.rng.gen(),
        };
        self.utilization.push(first);
        self.generate_point(first, last, LEVELS, LEVELS, cx);
        self.utilization.push(last);
    }
}

impl Entry for Summary {
    fn label_text(&self) -> &str {
        "avg"
    }
    fn hover_text(&self) -> &str {
        "Utilization Plot of Average Usage Over Time"
    }

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        _viewport: Rect,
        _config: &mut Config,
        cx: &mut Context,
    ) {
        const TOOLTIP_RADIUS: f32 = 4.0;
        let response = ui.allocate_rect(rect, egui::Sense::hover());
        let hover_pos = response.hover_pos(); // where is the mouse hovering?

        if self.utilization.is_empty() {
            self.generate(cx);
        }

        let style = ui.style();
        let visuals = style.interact_selectable(&response, false);
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);

        let stroke = Stroke::new(visuals.bg_stroke.width, self.color);

        let mut last_util = None;
        let mut hover_util = None;
        let mut last_point = None;
        let mut last_last_point: Option<Pos2> = None;
        for util in &self.utilization {
            // Convert utilization to screen space
            let point = rect.lerp(Vec2::new(util.time, 1.0 - util.util));
            if let Some(last) = last_point {
                ui.painter().line_segment([last, point], stroke);
            }

            if let (Some(hover), Some(last), Some(last_last)) =
                (hover_pos, last_point, last_last_point)
            {
                let point_dist = (point.x - hover.x).abs();
                let last_dist = (last.x - hover.x).abs();
                let last_last_dist = (last_last.x - hover.x).abs();
                if last_dist < point_dist && last_dist < last_last_dist {
                    ui.painter()
                        .circle_stroke(last, TOOLTIP_RADIUS, visuals.fg_stroke);
                    hover_util = last_util;
                }
            }

            last_last_point = last_point;
            last_point = Some(point);
            last_util = Some(util);
        }

        if let Some(util) = hover_util {
            let util_rect = Rect::from_min_max(
                rect.lerp(Vec2::new(util.time - 0.05, 0.0)),
                rect.lerp(Vec2::new(util.time + 0.05, 1.0)),
            );
            let util_response = ui.allocate_rect(util_rect, egui::Sense::hover());
            util_response.on_hover_text(format!("{:.0}% Utilization", util.util * 100.0));
        }
    }

    fn height(&self, _config: &Config, cx: &Context) -> f32 {
        const ROWS: u64 = 4;
        ROWS as f32 * cx.row_height
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

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        viewport: Rect,
        _config: &mut Config,
        _cx: &mut Context,
    ) {
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

    fn height(&self, _config: &Config, cx: &Context) -> f32 {
        self.rows() as f32 * cx.row_height
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
        config: &mut Config,
        cx: &mut Context,
    ) -> bool {
        const LABEL_WIDTH: f32 = 60.0;
        const COL_PADDING: f32 = 4.0;
        const ROW_PADDING: f32 = 4.0;

        // Compute the size of this slot
        // This is in screen (i.e., rect) space
        let min_y = *y;
        let max_y = min_y + slot.height(config, cx);
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

        slot.content(ui, content_subrect, content_viewport, config, cx);
        slot.label(ui, label_subrect);

        false
    }

    fn is_slot_visible(parent_level: u64, index: u64, config: &Config) -> bool {
        parent_level != 0 || (index >= config.min_node && index <= config.max_node)
    }
}

impl<S: Entry> Entry for Panel<S> {
    fn label_text(&self) -> &str {
        &self.short_name
    }
    fn hover_text(&self) -> &str {
        &self.long_name
    }

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        viewport: Rect,
        config: &mut Config,
        cx: &mut Context,
    ) {
        let mut y = rect.min.y;
        if let Some(summary) = &mut self.summary {
            Self::render(ui, rect, viewport, summary, &mut y, config, cx);
        }

        if self.expanded {
            for (i, slot) in self.slots.iter_mut().enumerate() {
                // Apply visibility settings
                if !Self::is_slot_visible(self.level, i as u64, config) {
                    continue;
                }

                if Self::render(ui, rect, viewport, slot, &mut y, config, cx) {
                    break;
                }
            }
        }
    }

    fn height(&self, config: &Config, cx: &Context) -> f32 {
        const UNEXPANDED_ROWS: u64 = 2;
        const ROW_PADDING: f32 = 4.0;

        let mut total = 0.0;
        let mut rows: i64 = 0;
        if let Some(summary) = &self.summary {
            total += summary.height(config, cx);
            rows += 1;
        } else if !self.expanded {
            // Need some minimum space if this panel has no summary and is collapsed
            total += UNEXPANDED_ROWS as f32 * cx.row_height;
            rows += 1;
        }

        if self.expanded {
            for (i, slot) in self.slots.iter().enumerate() {
                // Apply visibility settings
                if !Self::is_slot_visible(self.level, i as u64, config) {
                    continue;
                }

                total += slot.height(config, cx);
                rows += 1;
            }
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

impl Window {
    fn content(&mut self, ui: &mut egui::Ui, cx: &mut Context) {
        if self.panel.slots.is_empty() {
            self.generate(cx);
        }

        ui.heading(format!("Profile {}", self.index));

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_viewport(ui, |ui, viewport| {
                let height = self.panel.height(&self.config, cx);
                ui.set_height(height);
                ui.set_width(ui.available_width());

                let rect = Rect::from_min_size(ui.min_rect().min, viewport.size());

                // Root panel has no label
                self.panel.content(ui, rect, viewport, &mut self.config, cx);

                // Draw vertical line through cursor
                let response = ui.allocate_rect(ui.min_rect(), egui::Sense::hover());
                if let Some(hover) = response.hover_pos() {
                    let visuals = ui.style().interact_selectable(&response, false);

                    const RADIUS: f32 = 12.0;
                    let top = Pos2::new(hover.x, ui.min_rect().min.y);
                    let mid_top =
                        Pos2::new(hover.x, (hover.y - RADIUS).at_least(ui.min_rect().min.y));
                    let mid_bottom =
                        Pos2::new(hover.x, (hover.y + RADIUS).at_most(ui.min_rect().max.y));
                    let bottom = Pos2::new(hover.x, ui.min_rect().max.y);
                    ui.painter().line_segment([top, mid_top], visuals.fg_stroke);
                    ui.painter()
                        .line_segment([mid_bottom, bottom], visuals.fg_stroke);
                }
            });
    }

    fn node_selection(&mut self, ui: &mut egui::Ui) {
        ui.heading("Node Selection");
        let total = self.panel.slots.len().saturating_sub(1) as u64;
        let min_node = &mut self.config.min_node;
        let max_node = &mut self.config.max_node;
        ui.add(Slider::new(min_node, 0..=total).text("First"));
        if *min_node > *max_node {
            *max_node = *min_node;
        }
        ui.add(Slider::new(max_node, 0..=total).text("Last"));
        if *min_node > *max_node {
            *min_node = *max_node;
        }
    }

    fn expand_collapse(&mut self, ui: &mut egui::Ui) {
        let mut toggle_all = |label, toggle| {
            for node in &mut self.panel.slots {
                for kind in &mut node.slots {
                    if kind.expanded == toggle && kind.label_text() == label {
                        kind.toggle_expanded();
                    }
                }
            }
        };

        ui.heading("Expand/Collapse");
        ui.label("Expand by kind:");
        ui.horizontal_wrapped(|ui| {
            for kind in &self.kinds {
                if ui.button(kind).clicked() {
                    toggle_all(kind.to_lowercase(), false);
                }
            }
        });
        ui.label("Collapse by kind:");
        ui.horizontal_wrapped(|ui| {
            for kind in &self.kinds {
                if ui.button(kind).clicked() {
                    toggle_all(kind.to_lowercase(), true);
                }
            }
        });
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        const WIDGET_PADDING: f32 = 8.0;
        ui.heading(format!("Profile {}: Controls", self.index));
        ui.add_space(WIDGET_PADDING);
        self.node_selection(ui);
        ui.add_space(WIDGET_PADDING);
        self.expand_collapse(ui);
    }

    fn generate(&mut self, cx: &mut Context) {
        self.kinds = vec![
            "CPU".to_string(),
            "GPU".to_string(),
            "OMP".to_string(),
            "Py".to_string(),
            "Util".to_string(),
            "Chan".to_string(),
            "SysMem".to_string(),
        ];

        const NODES: i32 = 8192;
        const PROCS: i32 = 8;
        let mut node_slots = Vec::new();
        for node in 0..NODES {
            let mut kind_slots = Vec::new();
            let colors = &[Color32::BLUE, Color32::GREEN, Color32::RED, Color32::YELLOW];
            for (i, kind) in self.kinds.iter().enumerate() {
                let color = colors[i % colors.len()];
                let mut proc_slots = Vec::new();
                for proc in 0..PROCS {
                    let rows: u64 = cx.rng.gen_range(0..64);
                    let items = Vec::new();
                    // Leave items empty, we'll generate it later
                    proc_slots.push(Slot {
                        expanded: true,
                        short_name: format!(
                            "{}{}",
                            kind.chars().next().unwrap().to_lowercase(),
                            proc
                        ),
                        long_name: format!("Node {} {} {}", node, kind, proc),
                        max_rows: rows,
                        items,
                    });
                }
                kind_slots.push(Panel {
                    expanded: false,
                    short_name: kind.to_lowercase(),
                    long_name: format!("Node {} {}", node, kind),
                    level: 2,
                    summary: Some(Summary {
                        utilization: Vec::new(),
                        color,
                    }),
                    slots: proc_slots,
                });
            }
            node_slots.push(Panel {
                expanded: true,
                short_name: format!("n{}", node),
                long_name: format!("Node {}", node),
                level: 1,
                summary: None,
                slots: kind_slots,
            });
        }
        self.panel = Panel {
            expanded: true,
            short_name: "root".to_owned(),
            long_name: "root".to_owned(),
            level: 0,
            summary: None,
            slots: node_slots,
        };
        self.config.min_node = 0;
        self.config.max_node = self.panel.slots.len() as u64 - 1;
    }
}

impl ProfApp {
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

        result.windows.clear();
        result.windows.push(Window::default());

        #[cfg(not(target_arch = "wasm32"))]
        {
            result.last_update = Some(Instant::now());
        }

        result
    }
}

impl eframe::App for ProfApp {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            windows,
            cx,
            #[cfg(not(target_arch = "wasm32"))]
            last_update,
            ..
        } = self;

        let mut _fps = 0.0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = Instant::now();
            if let Some(last) = last_update {
                _fps = 1.0 / now.duration_since(*last).as_secs_f64();
            }
            *last_update = Some(now);
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
            ui.heading("Legion Prof Tech Demo");

            const WIDGET_PADDING: f32 = 8.0;
            ui.add_space(WIDGET_PADDING);

            for window in windows.iter_mut() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    window.controls(ui);
                });
            }

            if ui.button("Add Another Profile").clicked() {
                let mut index = 0;
                if let Some(last) = windows.last() {
                    index = last.index + 1;
                }
                windows.push(Window::default());
                windows.last_mut().unwrap().index = index;
            }

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Task Details");
                ui.label("Click on a task to see it displayed here.");
            });

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

                egui::warn_if_debug_build(ui);

                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    ui.label(format!("FPS: {:.0}", _fps));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Use body font to figure out how tall to draw rectangles.
            let font_id = TextStyle::Body.resolve(ui.style());
            let row_height = ui.fonts().row_height(&font_id);
            // Just set this on every frame for now
            cx.row_height = row_height;

            let mut remaining = windows.len();
            // Only wrap in a frame if more than one profile
            if remaining > 1 {
                for window in windows.iter_mut() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.push_id(window.index, |ui| {
                            ui.set_height(ui.available_height() / (remaining as f32));
                            ui.set_width(ui.available_width());
                            window.content(ui, cx);
                            remaining -= 1;
                        });
                    });
                }
            } else {
                for window in windows.iter_mut() {
                    window.content(ui, cx);
                }
            }
        });
    }
}

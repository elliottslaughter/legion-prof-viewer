use egui::{Pos2, Rect, Vec2};
use rand::Rng;
use std::time::{Duration, Instant};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AppRect {
    r: Rect,
    v: Vec2,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: f32,

    rects: Vec<AppRect>,
    #[serde(skip)]
    last_update: Instant,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            rects: Vec::new(),
            last_update: Instant::now(),
        }
    }
}

impl TemplateApp {
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
        const N: i32 = 30000;
        for _ in 0..N {
            let x: f32 = rng.gen();
            let y: f32 = rng.gen();
            let sx: f32 = rng.gen();
            let sy: f32 = rng.gen();
            let vx: f32 = rng.gen();
            let vy: f32 = rng.gen();
            result.rects.push(AppRect {
                r: Rect::from_min_size(
                    Pos2::new(x * 7.0 / 8.0, y * 7.0 / 8.0),
                    Vec2::new(sx / 8.0, sy / 8.0),
                ),
                v: Vec2::new((vx - 0.5) * 0.1, (vy - 0.5) * 0.1),
            });
        }
        result.last_update = Instant::now();

        result
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            label,
            value,
            rects,
            last_update,
        } = self;

        for r in rects.iter_mut() {
            // FIXME: need to estimate frame rate
            r.r = r.r.translate(r.v / 60.0);
        }

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
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

            let now = Instant::now();
            ui.label(format!(
                "FPS: {:.0}",
                1.0 / now.duration_since(*last_update).as_secs_f64()
            ));
            *last_update = now;

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(label);
            });

            ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                *value += 1.0;
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
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("Test Heading");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            ui.add(viewer(rects));
            egui::warn_if_debug_build(ui);
        });
    }
}

pub fn viewer_ui(ui: &mut egui::Ui, rects: &Vec<AppRect>) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
    if response.clicked() {
        // This is the click handler
    }
    if ui.is_rect_visible(rect) {
        // Draw the widget
        let visuals = ui.style().interact_selectable(&response, false);
        // let rect = rect.expand(visuals.expansion);
        for r in rects {
            let r2 = Rect::from_min_max(
                rect.lerp(r.r.left_top().to_vec2()),
                rect.lerp(r.r.right_bottom().to_vec2()),
            );
            let r2 = r2.expand(visuals.expansion);
            ui.painter()
                .rect(r2, 0.0, visuals.bg_fill, visuals.bg_stroke);
        }
    }
    response
}

pub fn viewer(rects: &Vec<AppRect>) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| viewer_ui(ui, rects)
}

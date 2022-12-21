#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui::{Color32, NumExt};
use rand::Rng;

use legion_prof_viewer::data::{
    DataSource, EntryID, EntryInfo, Item, SlotTile, SummaryTile, TileID, UtilPoint,
};
use legion_prof_viewer::timestamp::{Interval, Timestamp};

fn main() {
    legion_prof_viewer::app::start(
        Box::<RandomDataSource>::default(),
        Some(Box::<RandomDataSource>::default()),
    );
}

#[derive(Default)]
struct RandomDataSource {
    info: Option<EntryInfo>,
    interval: Option<Interval>,
    rng: rand::rngs::ThreadRng,
}

impl RandomDataSource {
    fn generate_point(
        &mut self,
        first: UtilPoint,
        last: UtilPoint,
        level: i32,
        max_level: i32,
        utilization: &mut Vec<UtilPoint>,
    ) {
        let time = Timestamp((first.time.0 + last.time.0) / 2);
        let util = (first.util + last.util) * 0.5;
        let diff = (self.rng.gen::<f32>() - 0.5) / 1.2_f32.powi(max_level - level);
        let util = (util + diff).at_least(0.0).at_most(1.0);
        let point = UtilPoint { time, util };
        if level > 0 {
            self.generate_point(first, point, level - 1, max_level, utilization);
        }
        utilization.push(point);
        if level > 0 {
            self.generate_point(point, last, level - 1, max_level, utilization);
        }
    }
}

const TILES: i64 = 3;

impl DataSource for RandomDataSource {
    fn interval(&mut self) -> Interval {
        if let Some(interval) = self.interval {
            return interval;
        }
        let interval = Interval::new(
            Timestamp(0),
            Timestamp(self.rng.gen_range(1_000_000..2_000_000)),
        );
        self.interval = Some(interval);
        interval
    }

    fn fetch_info(&mut self) -> &EntryInfo {
        if let Some(ref info) = self.info {
            return info;
        }

        let kinds = vec![
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
            for (i, kind) in kinds.iter().enumerate() {
                let color = colors[i % colors.len()];
                let mut proc_slots = Vec::new();
                for proc in 0..PROCS {
                    let rows: u64 = self.rng.gen_range(0..64);
                    proc_slots.push(EntryInfo::Slot {
                        short_name: format!(
                            "{}{}",
                            kind.chars().next().unwrap().to_lowercase(),
                            proc
                        ),
                        long_name: format!("Node {} {} {}", node, kind, proc),
                        max_rows: rows,
                    });
                }
                kind_slots.push(EntryInfo::Panel {
                    short_name: kind.to_lowercase(),
                    long_name: format!("Node {} {}", node, kind),
                    summary: Some(Box::new(EntryInfo::Summary { color })),
                    slots: proc_slots,
                });
            }
            node_slots.push(EntryInfo::Panel {
                short_name: format!("n{}", node),
                long_name: format!("Node {}", node),
                summary: None,
                slots: kind_slots,
            });
        }
        self.info = Some(EntryInfo::Panel {
            short_name: "root".to_owned(),
            long_name: "root".to_owned(),
            summary: None,
            slots: node_slots,
        });
        self.info.as_ref().unwrap()
    }

    fn request_tiles(&mut self, _entry_id: &EntryID, request_interval: Interval) -> Vec<TileID> {
        let duration = request_interval.duration_ns();

        let mut tiles = Vec::new();
        for i in 0..TILES {
            let start = Timestamp(i * duration / TILES + request_interval.start.0);
            let stop = Timestamp((i + 1) * duration / TILES + request_interval.start.0 - 1);
            tiles.push(TileID(Interval::new(start, stop)));
        }
        tiles
    }

    fn fetch_summary_tile(&mut self, _entry_id: &EntryID, tile_id: TileID) -> SummaryTile {
        const LEVELS: i32 = 8;
        let first = UtilPoint {
            time: tile_id.0.start,
            util: self.rng.gen(),
        };
        let last = UtilPoint {
            time: tile_id.0.stop,
            util: self.rng.gen(),
        };
        let mut utilization = Vec::new();
        utilization.push(first);
        self.generate_point(first, last, LEVELS, LEVELS, &mut utilization);
        utilization.push(last);

        SummaryTile {
            tile_id,
            utilization,
        }
    }

    fn fetch_slot_tile(&mut self, entry_id: &EntryID, tile_id: TileID) -> SlotTile {
        let entry = self.fetch_info().get(entry_id);

        let max_rows = if let EntryInfo::Slot { max_rows, .. } = entry.unwrap() {
            max_rows
        } else {
            panic!("trying to fetch tile on something that is not a slot")
        };

        let mut items = Vec::new();
        for row in 0..*max_rows {
            let mut row_items = Vec::new();
            const N: u64 = 1000 / (TILES as u64);
            for i in 0..N {
                let start = tile_id.0.lerp((i as f32 + 0.05) / (N as f32));
                let stop = tile_id.0.lerp((i as f32 + 0.95) / (N as f32));

                let color = match (row * N + i) % 7 {
                    0 => Color32::BLUE,
                    1 => Color32::GREEN,
                    2 => Color32::RED,
                    3 => Color32::YELLOW,
                    4 => Color32::KHAKI,
                    5 => Color32::DARK_GREEN,
                    6 => Color32::DARK_BLUE,
                    _ => Color32::WHITE,
                };

                row_items.push(Item {
                    interval: Interval::new(start, stop),
                    color,
                });
            }
            items.push(row_items);
        }
        SlotTile { tile_id, items }
    }
}

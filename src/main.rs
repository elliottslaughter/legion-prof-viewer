#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use rand::Rng;

use legion_prof_viewer::data::{DataSource, SlotTree};

fn main() {
    legion_prof_viewer::app::start();
}

#[derive(Default)]
struct RandomDataSource {
    tree: Option<SlotTree>
    rng: rand::rngs::ThreadRng,
}

impl DataSource for RandomDataSource {
    fn interval(&mut self) -> Interval {
        config.interval = Interval::new(
            Timestamp(0),
            Timestamp(result.cx.rng.gen_range(1_000_000..2_000_000)),
        );
    }
    fn fetch_info(&mut self) -> &EntryInfo {
        if let Some(tree) = self.slot_tree {
            return tree;
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
            for (i, kind) in self.kinds.iter().enumerate() {
                let color = colors[i % colors.len()];
                let mut proc_slots = Vec::new();
                for proc in 0..PROCS {
                    let rows: u64 = cx.rng.gen_range(0..64);
                    let items = Vec::new();
                    // Leave items empty, we'll generate it later
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
                    summary: Some(EntryInfo::Summary { color }),
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
        self.slot_tree = Some(EntryInfo::Panel {
            short_name: "root".to_owned(),
            long_name: "root".to_owned(),
            summary: None,
            slots: node_slots,
        });
        self.slot_tree
    }

    fn select_tiles(&mut self, entry: &EntryID, request_interval: Interval) -> Vec<TileID> {
    }

    fn generate_point(
        &mut self,
        first: UtilPoint,
        last: UtilPoint,
        level: i32,
        max_level: i32,
        cx: &mut Context,
    ) {
        let time = Timestamp((first.time.0 + last.time.0) / 2);
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

    fn fetch_summary_tile(&mut self, entry: &EntryID, tile: &TileID) -> SummaryTile {
        const LEVELS: i32 = 8;
        let first = UtilPoint {
            time: config.interval.start,
            util: self.rng.gen(),
        };
        let last = UtilPoint {
            time: config.interval.stop,
            util: self.rng.gen(),
        };
        self.utilization.push(first);
        self.generate_point(first, last, LEVELS, LEVELS, cx);
        self.utilization.push(last);
    }

    fn fetch_slot_tile(&mut self, entry: &EntryID, tile: &TileID) -> SlotTile {
    }
}

use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::timestamp::Interval;

#[derive(Deserialize, Serialize)]
pub struct SlotID(Vec<u64>);

#[derive(Deserialize, Serialize)]
pub enum SlotTree {
    Group {
        slots: Vec<SlotTree>,
        short_name: String,
        long_name: String,
    },
    Slot {
        short_name: String,
        long_name: String,
    },
}

pub struct Item {
    pub interval: Interval,
    pub color: Color32,
}

#[derive(Deserialize, Serialize)]
pub struct TileID(Interval);

pub struct Tile {
    pub items: Vec<Vec<Item>>, // row -> [item]
}

trait DataSource {
    fn fetch_tree(&mut self) -> SlotTree;
    fn select_tiles(&mut self, slot: &SlotID, request_interval: Interval) -> Vec<TileID>;
    fn fetch_tile(&mut self, slot: &SlotID, tile: &TileID) -> Tile;
}

use egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::timestamp::{Interval, Timestamp};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntryID(Vec<u64>);

impl EntryID {
    pub fn root() -> Self {
        Self(Vec::new())
    }
    pub fn child(&self, index: u64) -> Self {
        let mut result = self.clone();
        result.0.push(index);
        result
    }
}

#[derive(Deserialize, Serialize)]
pub enum EntryInfo {
    Panel {
        short_name: String,
        long_name: String,
        summary: Option<Box<EntryInfo>>,
        slots: Vec<EntryInfo>,
    },
    Slot {
        short_name: String,
        long_name: String,
        max_rows: u64,
    },
    Summary {
        color: Color32,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct UtilPoint {
    pub time: Timestamp,
    pub util: f32,
}

pub struct Item {
    pub interval: Interval,
    pub color: Color32,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct TileID(pub Interval);

pub struct SummaryTile {
    pub utilization: Vec<UtilPoint>,
}

pub struct SlotTile {
    pub items: Vec<Vec<Item>>, // row -> [item]
}

pub trait DataSource {
    fn interval(&mut self) -> Interval;
    fn fetch_info(&mut self) -> &EntryInfo;
    fn request_tiles(&mut self, entry: &EntryID, request_interval: Interval) -> Vec<TileID>;
    fn fetch_summary_tile(&mut self, entry: &EntryID, tile: &TileID) -> SummaryTile;
    fn fetch_slot_tile(&mut self, entry: &EntryID, tile: &TileID) -> SlotTile;
}

impl EntryInfo {
    pub fn nodes(&self) -> u64 {
        if let EntryInfo::Panel { slots, .. } = self {
            slots.len() as u64
        } else {
            unreachable!()
        }
    }
    pub fn kinds(&self) -> Vec<String> {
        if let EntryInfo::Panel { slots: nodes, .. } = self {
            let mut result = Vec::new();
            let mut set = BTreeSet::new();
            for node in nodes {
                if let EntryInfo::Panel { slots: kinds, .. } = node {
                    for kind in kinds {
                        if let EntryInfo::Panel { short_name, .. } = kind {
                            if set.insert(short_name) {
                                result.push(short_name.clone());
                            }
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    unreachable!();
                }
            }
            return result;
        }
        unreachable!()
    }
}

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Timestamp(pub i64 /* ns */);

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Time is stored in nanoseconds. But display in larger units if possible.
        let ns = self.0;
        let ns_per_us = 1_000;
        let ns_per_ms = 1_000_000;
        let ns_per_s = 1_000_000_000;
        let divisor;
        let remainder_divisor;
        let mut unit_name = "ns";
        if ns >= ns_per_s {
            divisor = ns_per_s;
            remainder_divisor = divisor / 1_000;
            unit_name = "s";
        } else if ns >= ns_per_ms {
            divisor = ns_per_ms;
            remainder_divisor = divisor / 1_000;
            unit_name = "ms";
        } else if ns >= ns_per_us {
            divisor = ns_per_us;
            remainder_divisor = divisor / 1_000;
            unit_name = "us";
        } else {
            return write!(f, "{} {}", ns, unit_name);
        }
        let units = ns / divisor;
        let remainder = (ns % divisor) / remainder_divisor;
        write!(f, "{}.{:0>3} {}", units, remainder, unit_name)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Interval {
    pub start: Timestamp,
    pub stop: Timestamp,
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Time is stored in nanoseconds. But display in larger units if possible.
        let start_ns = self.start.0;
        let stop_ns = self.stop.0;
        let ns_per_us = 1_000;
        let ns_per_ms = 1_000_000;
        let ns_per_s = 1_000_000_000;
        let divisor;
        let remainder_divisor;
        let mut unit_name = "ns";
        if stop_ns >= ns_per_s {
            divisor = ns_per_s;
            remainder_divisor = divisor / 1_000;
            unit_name = "s";
        } else if stop_ns >= ns_per_ms {
            divisor = ns_per_ms;
            remainder_divisor = divisor / 1_000;
            unit_name = "ms";
        } else if stop_ns >= ns_per_us {
            divisor = ns_per_us;
            remainder_divisor = divisor / 1_000;
            unit_name = "us";
        } else {
            return write!(
                f,
                "from {} to {} {} (duration: {})",
                start_ns,
                stop_ns,
                unit_name,
                Timestamp(stop_ns - start_ns)
            );
        }
        let start_units = start_ns / divisor;
        let start_remainder = (start_ns % divisor) / remainder_divisor;
        let stop_units = stop_ns / divisor;
        let stop_remainder = (stop_ns % divisor) / remainder_divisor;
        write!(
            f,
            "from {}.{:0>3} to {}.{:0>3} {} (duration: {})",
            start_units,
            start_remainder,
            stop_units,
            stop_remainder,
            unit_name,
            Timestamp(stop_ns - start_ns)
        )
    }
}

impl Interval {
    pub fn new(start: Timestamp, stop: Timestamp) -> Self {
        Self { start, stop }
    }
    pub fn duration_ns(self) -> i64 {
        self.stop.0 - self.start.0
    }
    pub fn contains(self, point: Timestamp) -> bool {
        point >= self.start && point <= self.stop
    }
    pub fn overlaps(self, other: Interval) -> bool {
        !(other.stop < self.start || other.start > self.stop)
    }
    pub fn intersection(self, other: Interval) -> Self {
        Self {
            start: Timestamp(self.start.0.max(other.start.0)),
            stop: Timestamp(self.stop.0.min(other.stop.0)),
        }
    }
    pub fn union(self, other: Interval) -> Self {
        Self {
            start: Timestamp(self.start.0.min(other.start.0)),
            stop: Timestamp(self.stop.0.max(other.stop.0)),
        }
    }
    // Convert a timestamp into [0,1] relative space
    pub fn unlerp(self, time: Timestamp) -> f32 {
        (time.0 - self.start.0) as f32 / (self.duration_ns() as f32)
    }
    // Convert [0,1] relative space into a timestamp
    pub fn lerp(self, value: f32) -> Timestamp {
        Timestamp((value * (self.duration_ns() as f32)).round() as i64 + self.start.0)
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single data point for a bus update.
/// Optimized for size: Interval and Latency are u16 (max ~18 hours).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Record {
    pub interval: u16,        // Seconds
    pub end_of_interval: u64, // Unix timestamp
    pub latency: u16,         // Seconds
    pub rank: u8,             // Percentile rank (0-100)
}

/// System-wide statistics for a specific minute.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemStats {
    pub timestamp: u64, // End of the minute (Unix timestamp)
    pub interval_stats: Percentiles,
    pub latency_stats: Percentiles,
    pub sample_count: u32,
}

/// Percentiles for a distribution.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Percentiles {
    pub p0: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p80: f64,
    pub p85: f64,
    pub p90: f64,
    pub p95: f64,
    pub p98: f64,
    pub p99: f64,
    pub p99_5: f64,
    pub p99_9: f64,
}

impl Percentiles {
    pub fn get_rank(&self, value: f64) -> u8 {
        if value < self.p0 {
            return 0;
        }
        if value < self.p25 {
            return 25;
        } // Approximate
        if value < self.p50 {
            return 50;
        }
        if value < self.p75 {
            return 75;
        }
        if value < self.p80 {
            return 80;
        }
        if value < self.p85 {
            return 85;
        }
        if value < self.p90 {
            return 90;
        }
        if value < self.p95 {
            return 95;
        }
        if value < self.p98 {
            return 98;
        }
        if value < self.p99 {
            return 99;
        }
        if value < self.p99_5 {
            return 99;
        } // bucket into 99
        if value < self.p99_9 {
            return 100;
        }
        100
    }
}

/// The file format for a 1-minute data chunk.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChunkFile {
    pub stats: SystemStats,
    pub records: HashMap<String, Vec<Record>>,
}

// Ensure the data directory exists
pub fn ensure_data_dir() -> std::io::Result<()> {
    use std::fs;
    if !fs::metadata("./data").is_ok() {
        fs::create_dir("./data")?;
    }
    Ok(())
}

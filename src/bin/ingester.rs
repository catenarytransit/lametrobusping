use lametrobusping::{ChunkFile, Percentiles, Record, SystemStats, ensure_data_dir};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;

const GTFS_URL: &str = "https://birch.catenarymaps.org/gtfs_rt?feed_id=f-metro~losangeles~bus~rt&feed_type=vehicle&format=json";
const RETENTION_Seconds: u64 = 48 * 3600;

#[derive(Deserialize, Debug)]
struct GtfsFeed {
    header: Header,
    entity: Vec<Entity>,
}

#[derive(Deserialize, Debug)]
struct Header {
    timestamp: u64,
}

#[derive(Deserialize, Debug)]
struct Entity {
    id: String,
    vehicle: Option<VehiclePosition>,
}

#[derive(Deserialize, Debug)]
struct VehiclePosition {
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_data_dir()?;
    let client = reqwest::Client::new();

    // State
    let mut last_updates: HashMap<String, u64> = HashMap::new();
    let mut current_chunk: HashMap<String, Vec<Record>> = HashMap::new();
    let mut interval_samples: Vec<u16> = Vec::new();
    let mut latency_samples: Vec<u16> = Vec::new();
    let mut last_flush_time = SystemTime::now();

    let mut interval_timer = time::interval(Duration::from_secs(1));

    println!("Starting ingester...");

    loop {
        interval_timer.tick().await;

        match fetch_feed(&client).await {
            Ok(feed) => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                let dataset_ts = feed.header.timestamp;

                for entity in feed.entity {
                    if let Some(vehicle) = entity.vehicle {
                        let bus_id = entity.id;
                        let vehicle_ts = vehicle.timestamp;

                        // Only process if we have seen this bus before
                        if let Some(&prev_ts) = last_updates.get(&bus_id) {
                            if vehicle_ts > prev_ts {
                                let interval_raw = vehicle_ts - prev_ts;
                                let latency_raw = if dataset_ts > vehicle_ts {
                                    dataset_ts - vehicle_ts
                                } else {
                                    0
                                };

                                let interval = interval_raw.min(u16::MAX as u64) as u16;
                                let latency = latency_raw.min(u16::MAX as u64) as u16;

                                // Add to current chunk
                                let record = Record {
                                    interval,
                                    end_of_interval: vehicle_ts,
                                    latency,
                                    rank: 0, // Placeholder, updated before flush
                                };
                                current_chunk
                                    .entry(bus_id.clone())
                                    .or_default()
                                    .push(record);

                                // Add to stats samples
                                interval_samples.push(interval);
                                latency_samples.push(latency);
                            }
                        }

                        // Always update last seen timestamp if it's newer
                        if let Some(&prev_ts) = last_updates.get(&bus_id) {
                            if vehicle_ts > prev_ts {
                                last_updates.insert(bus_id, vehicle_ts);
                            }
                        } else {
                            last_updates.insert(bus_id, vehicle_ts);
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error fetching feed: {}", e),
        }

        // Check flush (every 60s)
        if last_flush_time.elapsed()? >= Duration::from_secs(60) {
            let chunk_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

            // Calc stats
            let stats = SystemStats {
                timestamp: chunk_ts,
                interval_stats: calc_percentiles(&mut interval_samples),
                latency_stats: calc_percentiles(&mut latency_samples),
                sample_count: interval_samples.len() as u32, // interval and latency have same count
            };

            // Update ranks in current_chunk
            for records in current_chunk.values_mut() {
                for record in records {
                    record.rank = stats.interval_stats.get_rank(record.interval as f64);
                }
            }

            let chunk_file = ChunkFile {
                stats,
                records: current_chunk.clone(),
            };

            // Write file
            let params = bincode::serialize(&chunk_file)?;
            let filename = format!("./data/chunk_{}.bin", chunk_ts);
            std::fs::write(&filename, params)?;
            println!(
                "Flushed chunk: {} ({} records)",
                filename,
                interval_samples.len()
            );

            // Reset
            current_chunk.clear();
            interval_samples.clear();
            latency_samples.clear();
            last_flush_time = SystemTime::now();

            // Prune old files
            prune_files(chunk_ts)?;
        }
    }
}

async fn fetch_feed(client: &reqwest::Client) -> Result<GtfsFeed, reqwest::Error> {
    client.get(GTFS_URL).send().await?.json::<GtfsFeed>().await
}

fn calc_percentiles(samples: &mut Vec<u16>) -> Percentiles {
    if samples.is_empty() {
        return Percentiles {
            p0: 0.0,
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
            p80: 0.0,
            p85: 0.0,
            p90: 0.0,
            p95: 0.0,
            p98: 0.0,
            p99: 0.0,
            p99_5: 0.0,
            p99_9: 0.0,
        };
    }
    samples.sort_unstable();
    let len = samples.len() as f64;

    let get_p = |p: f64| -> f64 {
        let idx = ((len - 1.0) * p).round() as usize;
        samples[idx] as f64
    };

    Percentiles {
        p0: get_p(0.0),
        p25: get_p(0.25),
        p50: get_p(0.50),
        p75: get_p(0.75),
        p80: get_p(0.80),
        p85: get_p(0.85),
        p90: get_p(0.90),
        p95: get_p(0.95),
        p98: get_p(0.98),
        p99: get_p(0.99),
        p99_5: get_p(0.995),
        p99_9: get_p(0.999),
    }
}

fn prune_files(current_ts: u64) -> std::io::Result<()> {
    // Delete files older than 48h
    let cutoff = current_ts.saturating_sub(RETENTION_Seconds);

    for entry in glob::glob("./data/chunk_*.bin").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(ts_str) = stem.strip_prefix("chunk_") {
                        if let Ok(ts) = ts_str.parse::<u64>() {
                            if ts < cutoff {
                                let _ = std::fs::remove_file(path);
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error reading glob entry: {}", e),
        }
    }
    Ok(())
}

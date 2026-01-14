use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use clap::Parser;
use lametrobusping::{ChunkFile, Record, SystemStats, ensure_data_dir};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time;

const RETENTION_SECONDS: u64 = 48 * 3600;

#[derive(Clone)]
struct AppState {
    // BusID -> History (Oldest first)
    history: Arc<RwLock<HashMap<String, VecDeque<Record>>>>,
    // System Stats History (Oldest first)
    stats: Arc<RwLock<VecDeque<SystemStats>>>,
    // Last loaded chunk timestamp to avoid reloading
    last_loaded_ts: Arc<RwLock<u64>>,
    // Rank (0-100) -> List of (Timestamp, BusId)
    anomalies: Arc<RwLock<HashMap<u8, VecDeque<(u64, String)>>>>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[derive(serde::Deserialize)]
struct AnomaliesQuery {
    min_rank: Option<u8>,
    // Optional: filter by time range if needed, e.g., since: Option<u64>
}

#[derive(serde::Serialize)]
struct ScoredBus {
    bus_id: String,
    score: u64,
    history: Vec<Record>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    ensure_data_dir()?;
    println!("Starting API...");

    let state = AppState {
        history: Arc::new(RwLock::new(HashMap::new())),
        stats: Arc::new(RwLock::new(VecDeque::new())),
        last_loaded_ts: Arc::new(RwLock::new(0)),
        anomalies: Arc::new(RwLock::new(HashMap::new())),
    };

    // Initial load
    load_recent_chunks(state.clone())?;

    // Background watcher
    let watcher_state = state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = load_recent_chunks(watcher_state.clone()) {
                eprintln!("Error loading chunks: {}", e);
            }
            if let Err(e) = prune_memory(watcher_state.clone()) {
                eprintln!("Error pruning memory: {}", e);
            }
        }
    });

    let app = Router::new()
        .route("/history/:bus_id", get(get_history))
        .route("/stats", get(get_stats))
        .route("/anomalies", get(get_anomalies))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

fn load_recent_chunks(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = glob::glob("./data/chunk_*.bin")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    // Sort by filename (timestamp)
    files.sort();

    let last_loaded = *state.last_loaded_ts.read().unwrap();

    for path in files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Some(ts_str) = stem.strip_prefix("chunk_") {
                if let Ok(ts) = ts_str.parse::<u64>() {
                    if ts > last_loaded {
                        // Load this file
                        let data = std::fs::read(&path)?;
                        let chunk_res: Result<ChunkFile, _> = bincode::deserialize(&data);

                        if let Ok(chunk) = chunk_res {
                            // Merge into state
                            {
                                let mut history = state.history.write().unwrap();
                                let mut anomalies = state.anomalies.write().unwrap();
                                for (bus_id, records) in chunk.records {
                                    let entry = history.entry(bus_id.clone()).or_default();
                                    for record in records {
                                        // Indexing
                                        anomalies
                                            .entry(record.rank)
                                            .or_default()
                                            .push_back((record.end_of_interval, bus_id.clone()));
                                        entry.push_back(record);
                                    }
                                }
                            }

                            {
                                let mut stats = state.stats.write().unwrap();
                                stats.push_back(chunk.stats);
                            }

                            *state.last_loaded_ts.write().unwrap() = ts;
                            println!("Loaded chunk: {}", ts);
                        } else {
                            eprintln!(
                                "Failed to deserialize chunk {}, skipping (schema mismatch?)",
                                ts
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn prune_memory(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let cutoff = now.saturating_sub(RETENTION_SECONDS);

    // Prune Stats
    {
        let mut stats = state.stats.write().unwrap();
        while let Some(front) = stats.front() {
            if front.timestamp < cutoff {
                stats.pop_front();
            } else {
                break;
            }
        }
    }

    // Prune History
    // This is expensive if done fully every 10s.
    // Optimization: Only check the front of each deque.
    {
        let mut history = state.history.write().unwrap();
        // Remove empty buses?
        let mut empty_buses = Vec::new();

        for (bus_id, records) in history.iter_mut() {
            while let Some(front) = records.front() {
                if front.end_of_interval < cutoff {
                    records.pop_front();
                } else {
                    break;
                }
            }
            if records.is_empty() {
                empty_buses.push(bus_id.clone());
            }
        }

        for bus_id in empty_buses {
            history.remove(&bus_id);
        }
    }

    // Prune Anomalies
    {
        let mut anomalies = state.anomalies.write().unwrap();
        for list in anomalies.values_mut() {
            while let Some(front) = list.front() {
                if front.0 < cutoff {
                    list.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn get_history(
    State(state): State<AppState>,
    Path(bus_id): Path<String>,
) -> Json<Vec<Record>> {
    let history = state.history.read().unwrap();
    if let Some(records) = history.get(&bus_id) {
        Json(records.iter().cloned().collect())
    } else {
        Json(Vec::new())
    }
}

async fn get_stats(State(state): State<AppState>) -> Json<Vec<SystemStats>> {
    let stats = state.stats.read().unwrap();
    Json(stats.iter().cloned().collect())
}

async fn get_anomalies(
    State(state): State<AppState>,
    Query(query): Query<AnomaliesQuery>,
) -> Json<Vec<ScoredBus>> {
    let min_rank = query.min_rank.unwrap_or(90);
    let anomalies = state.anomalies.read().unwrap();
    let history_lock = state.history.read().unwrap();

    let mut candidate_buses = std::collections::HashSet::new();

    // 1. Gather candidates
    for rank in min_rank..=100 {
        if let Some(list) = anomalies.get(&rank) {
            for (_, bus_id) in list {
                candidate_buses.insert(bus_id);
            }
        }
    }

    // 2. Score candidates
    let mut scored_buses = Vec::new();
    for bus_id in candidate_buses {
        if let Some(records) = history_lock.get(bus_id) {
            let mut score: u64 = 0;
            for record in records {
                if record.rank >= min_rank {
                    score += record.interval as u64;
                }
            }
            if score > 0 {
                scored_buses.push(ScoredBus {
                    bus_id: bus_id.clone(),
                    score,
                    history: records.iter().cloned().collect(),
                });
            }
        }
    }

    // 3. Sort by score descending
    scored_buses.sort_by(|a, b| b.score.cmp(&a.score));

    // Limit to top 50
    scored_buses.truncate(50);

    Json(scored_buses)
}

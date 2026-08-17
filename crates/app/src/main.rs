// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ratio_spoof_core::{
    RatioSpoofer, SpooferConfig, SpooferState, TorrentInfo,
    ClientEmulation,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tauri::{Emitter, State};

#[derive(Default)]
struct AppState {
    stop_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParsedTorrentInfo {
    name: String,
    total_size: u64,
    piece_length: u64,
    num_trackers: usize,
}

impl From<&TorrentInfo> for ParsedTorrentInfo {
    fn from(info: &TorrentInfo) -> Self {
        Self {
            name: info.name.clone(),
            total_size: info.total_size,
            piece_length: info.piece_length,
            num_trackers: info.trackers.len(),
        }
    }
}

#[tauri::command]
async fn parse_torrent(path: String) -> Result<ParsedTorrentInfo, String> {
    let torrent = ratio_spoof_core::torrent::parse_torrent_file(&path)
        .map_err(|e| e.to_string())?;

    Ok(ParsedTorrentInfo::from(&torrent))
}

#[tauri::command]
fn get_available_clients() -> Vec<(String, String)> {
    ClientEmulation::available_profiles()
        .into_iter()
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect()
}

#[tauri::command]
async fn start_spoofing(
    config: SpooferConfig,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Create spoofer instance
    let mut spoofer = RatioSpoofer::new(config)
        .await
        .map_err(|e| e.to_string())?;

    // Create channels
    let (state_tx, mut state_rx) = mpsc::channel::<SpooferState>(100);
    let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

    spoofer.set_stop_signal(stop_rx);

    // Store stop tx
    *state.stop_tx.lock().await = Some(stop_tx);

    // Spawn task to forward state updates to frontend
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(spoofer_state) = state_rx.recv().await {
            app_handle_clone.emit("spoofer-state", &spoofer_state).ok();
        }
    });

    // Start spoofing in background
    tokio::spawn(async move {
        if let Err(e) = spoofer.start(state_tx).await {
            eprintln!("Spoofer error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_spoofing(state: State<'_, AppState>) -> Result<(), String> {
    let stop_tx = state.stop_tx.lock().await.take();

    if let Some(tx) = stop_tx {
        tx.send(()).await.ok();
    }

    Ok(())
}

#[tauri::command]
fn parse_size_input(input: String, total_size: u64) -> Result<u64, String> {
    let input = input.trim().to_lowercase();

    // Check for percentage
    if let Some(percent_str) = input.strip_suffix('%') {
        let percent: f64 = percent_str.parse().map_err(|e| format!("Invalid percentage: {}", e))?;
        return Ok(((total_size as f64) * (percent / 100.0)) as u64);
    }

    // Check for units
    let (value_str, multiplier) = if let Some(val) = input.strip_suffix("tb") {
        (val, 1024u64 * 1024 * 1024 * 1024)
    } else if let Some(val) = input.strip_suffix("gb") {
        (val, 1024u64 * 1024 * 1024)
    } else if let Some(val) = input.strip_suffix("mb") {
        (val, 1024u64 * 1024)
    } else if let Some(val) = input.strip_suffix("kb") {
        (val, 1024u64)
    } else if let Some(val) = input.strip_suffix('b') {
        (val, 1u64)
    } else {
        // Assume bytes
        (&input[..], 1u64)
    };

    let value: f64 = value_str.parse().map_err(|e| format!("Invalid number: {}", e))?;
    Ok((value * multiplier as f64) as u64)
}

#[tauri::command]
fn parse_speed_input(input: String) -> Result<u64, String> {
    let input = input.trim().to_lowercase();

    // Check for units (speed is in bits per second, convert to bytes per second)
    let (value_str, multiplier) = if let Some(val) = input.strip_suffix("mbps") {
        (val, 1024u64 * 1024 / 8)
    } else if let Some(val) = input.strip_suffix("kbps") {
        (val, 1024u64 / 8)
    } else if let Some(val) = input.strip_suffix("mb/s") {
        (val, 1024u64 * 1024)
    } else if let Some(val) = input.strip_suffix("kb/s") {
        (val, 1024u64)
    } else {
        // Assume KB/s
        (&input[..], 1024u64)
    };

    let value: f64 = value_str.parse().map_err(|e| format!("Invalid number: {}", e))?;
    Ok((value * multiplier as f64) as u64)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            parse_torrent,
            get_available_clients,
            start_spoofing,
            stop_spoofing,
            parse_size_input,
            parse_speed_input,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

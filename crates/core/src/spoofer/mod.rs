use crate::{
    TorrentInfo,
    tracker::{TrackerClient, AnnounceRequest, AnnounceResponse},
    emulation::ClientEmulation,
    generator::{PeerIdGenerator, KeyGenerator, ValueRounder},
};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use chrono::{DateTime, Utc};
use rand::Rng;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpooferConfig {
    pub torrent_path: String,
    pub client_profile: String,
    pub initial_downloaded: u64,
    pub initial_uploaded: u64,
    pub download_speed: u64,  // bytes per second
    pub upload_speed: u64,    // bytes per second
    pub port: u16,
    // New features
    #[serde(default)]
    pub target_ratio: Option<f64>,           // Stop when this ratio is reached
    #[serde(default)]
    pub realistic_mode: bool,                // Enable speed variance
    #[serde(default = "default_variance")]
    pub variance_percent: f64,               // Variance percentage (e.g., 20.0 for ±20%)
    #[serde(default)]
    pub peer_rotation_minutes: Option<u64>,  // Rotate peer_id every N minutes
}

fn default_variance() -> f64 {
    20.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpooferState {
    pub status: SpooferStatus,
    pub current_downloaded: u64,
    pub current_uploaded: u64,
    pub current_left: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub next_announce_in: u64, // seconds
    pub announce_interval: u64, // seconds
    pub current_tracker: String,
    pub torrent_name: String,
    pub total_size: u64,
    pub history: Vec<ProgressUpdate>,
    // New feature states
    pub current_ratio: f64,
    pub target_ratio: Option<f64>,
    pub peer_rotation_count: u32,
    pub realistic_mode: bool,
    // Config display
    pub download_speed: u64,    // bytes per second
    pub upload_speed: u64,      // bytes per second
    pub variance_percent: f64,
    pub peer_id: String,
    pub client_profile: String,
    pub peer_rotation_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpooferStatus {
    Idle,
    Starting,
    Running,
    Paused,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub timestamp: DateTime<Utc>,
    pub downloaded: u64,
    pub uploaded: u64,
    pub left: u64,
    pub seeders: u32,
    pub leechers: u32,
}

pub struct RatioSpoofer {
    config: SpooferConfig,
    torrent: TorrentInfo,
    emulation: ClientEmulation,
    tracker: TrackerClient,
    peer_id: String,
    key: String,
    rounder: ValueRounder,
    state: SpooferState,
    history: Vec<ProgressUpdate>,
    stop_signal: Option<mpsc::Receiver<()>>,
    // Peer rotation tracking
    last_peer_rotation: Instant,
}

impl RatioSpoofer {
    /// Create a new RatioSpoofer instance
    pub async fn new(config: SpooferConfig) -> Result<Self> {
        // Parse torrent file
        let torrent = crate::torrent::parse_torrent_file(&config.torrent_path)
            .context("Failed to parse torrent file")?;

        // Load client emulation
        let emulation = ClientEmulation::load(&config.client_profile)
            .context("Failed to load client profile")?;

        // Generate peer ID and key
        let peer_id = PeerIdGenerator::generate(emulation.get_peer_id_regex())
            .context("Failed to generate peer ID")?;
        let key = KeyGenerator::generate();

        // Create tracker client
        let tracker = TrackerClient::new(torrent.trackers.clone());

        // Create value rounder
        let rounder = ValueRounder::new(torrent.piece_length);

        // Calculate initial left bytes
        let initial_left = torrent.total_size.saturating_sub(config.initial_downloaded);

        // Calculate initial ratio
        let initial_ratio = if config.initial_downloaded > 0 {
            config.initial_uploaded as f64 / config.initial_downloaded as f64
        } else {
            0.0
        };

        let state = SpooferState {
            status: SpooferStatus::Idle,
            current_downloaded: config.initial_downloaded,
            current_uploaded: config.initial_uploaded,
            current_left: initial_left,
            seeders: 0,
            leechers: 0,
            next_announce_in: 0,
            announce_interval: 1800,
            current_tracker: tracker.get_current_tracker().to_string(),
            torrent_name: torrent.name.clone(),
            total_size: torrent.total_size,
            history: Vec::new(),
            current_ratio: initial_ratio,
            target_ratio: config.target_ratio,
            peer_rotation_count: 0,
            realistic_mode: config.realistic_mode,
            download_speed: config.download_speed,
            upload_speed: config.upload_speed,
            variance_percent: config.variance_percent,
            peer_id: peer_id.clone(),
            client_profile: config.client_profile.clone(),
            peer_rotation_minutes: config.peer_rotation_minutes,
        };

        Ok(Self {
            config,
            torrent,
            emulation,
            tracker,
            peer_id,
            key,
            rounder,
            state,
            history: Vec::new(),
            stop_signal: None,
            last_peer_rotation: Instant::now(),
        })
    }

    pub fn get_state(&self) -> &SpooferState {
        &self.state
    }

    pub fn get_history(&self) -> &[ProgressUpdate] {
        &self.history
    }

    /// Start the spoofing process
    pub async fn start(&mut self, state_tx: mpsc::Sender<SpooferState>) -> Result<()> {
        self.state.status = SpooferStatus::Starting;
        state_tx.send(self.state.clone()).await.ok();

        eprintln!("[Spoofer] Starting announce to tracker: {}", self.tracker.get_current_tracker());
        eprintln!("[Spoofer] Info hash: {}", self.torrent.info_hash);
        eprintln!("[Spoofer] Peer ID: {}", self.peer_id);

        // Send initial "started" announce
        let response = match self.announce(Some("started")).await {
            Ok(r) => {
                eprintln!("[Spoofer] Announce successful! Interval: {}s, Seeders: {}, Leechers: {}",
                    r.interval, r.complete, r.incomplete);
                r
            }
            Err(e) => {
                eprintln!("[Spoofer] Announce FAILED: {}", e);
                self.state.status = SpooferStatus::Error(e.to_string());
                state_tx.send(self.state.clone()).await.ok();
                return Err(e);
            }
        };

        self.state.announce_interval = response.interval;
        self.state.seeders = response.complete;
        self.state.leechers = response.incomplete;
        self.state.status = SpooferStatus::Running;
        self.state.current_tracker = self.tracker.get_current_tracker().to_string();

        state_tx.send(self.state.clone()).await.ok();

        // Add to history
        self.add_to_history();

        // Main loop
        loop {
            self.state.next_announce_in = self.state.announce_interval;

            // Count down until next announce
            for remaining in (0..self.state.announce_interval).rev() {
                self.state.next_announce_in = remaining;
                state_tx.send(self.state.clone()).await.ok();

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // Check for stop signal
                if let Some(ref mut rx) = self.stop_signal {
                    if rx.try_recv().is_ok() {
                        return self.stop().await;
                    }
                }
            }

            // Generate next progress values
            self.generate_next_progress();

            // Check if target ratio reached
            if self.check_target_ratio() {
                eprintln!("[Spoofer] Target ratio {:.2} reached! Stopping...",
                    self.config.target_ratio.unwrap_or(0.0));
                return self.stop().await;
            }

            // Check if peer rotation is needed
            if let Err(e) = self.rotate_peer_if_needed().await {
                eprintln!("[Spoofer] Peer rotation error: {}", e);
            }

            // Send announce
            match self.announce(None).await {
                Ok(response) => {
                    self.state.announce_interval = response.interval;
                    self.state.seeders = response.complete;
                    self.state.leechers = response.incomplete;
                    self.state.current_tracker = self.tracker.get_current_tracker().to_string();
                    self.add_to_history();
                }
                Err(e) => {
                    eprintln!("Announce error: {}", e);
                    // Continue running despite errors
                }
            }

            state_tx.send(self.state.clone()).await.ok();
        }
    }

    /// Stop the spoofing process gracefully
    pub async fn stop(&mut self) -> Result<()> {
        self.state.status = SpooferStatus::Stopped;

        // Send "stopped" event
        self.announce(Some("stopped")).await?;

        Ok(())
    }

    /// Send an announce to the tracker
    async fn announce(&mut self, event: Option<&str>) -> Result<AnnounceResponse> {
        let (downloaded, uploaded, left) = self.rounder.round_values(
            self.state.current_downloaded,
            self.state.current_uploaded,
            self.state.current_left,
        );

        let request = AnnounceRequest {
            info_hash: self.torrent.info_hash.clone(),
            peer_id: self.peer_id.clone(),
            port: self.config.port,
            uploaded,
            downloaded,
            left,
            key: self.key.clone(),
            event: event.map(|s| s.to_string()),
            numwant: if event == Some("stopped") { 0 } else { 200 },
        };

        let query_template = self.emulation.get_query_template();
        let headers = self.emulation.get_headers();

        // retry=true for regular announces, false for started/stopped
        let retry = event.is_none();
        self.tracker.announce(&request, query_template, headers, retry).await
    }

    /// Generate next progress values based on configured speeds
    fn generate_next_progress(&mut self) {
        let interval_secs = self.state.announce_interval;
        let mut rng = rand::thread_rng();

        // Apply realistic mode variance if enabled
        let (download_speed, upload_speed) = if self.config.realistic_mode {
            let variance = self.config.variance_percent / 100.0;
            let dl_factor = 1.0 + rng.gen_range(-variance..=variance);
            let ul_factor = 1.0 + rng.gen_range(-variance..=variance);
            (
                (self.config.download_speed as f64 * dl_factor) as u64,
                (self.config.upload_speed as f64 * ul_factor) as u64,
            )
        } else {
            (self.config.download_speed, self.config.upload_speed)
        };

        // Calculate bytes to add
        let download_bytes = download_speed * interval_secs;
        let upload_bytes = upload_speed * interval_secs;

        // Add random pieces for realism (1-10 pieces)
        let download_bytes = self.rounder.add_random_pieces(download_bytes, (1, 10));
        let upload_bytes = self.rounder.add_random_pieces(upload_bytes, (1, 10));

        // Update downloaded (stop at 100%)
        let new_downloaded = (self.state.current_downloaded + download_bytes)
            .min(self.torrent.total_size);

        // Update uploaded (unlimited)
        let new_uploaded = self.state.current_uploaded + upload_bytes;

        // Update left
        let new_left = self.torrent.total_size.saturating_sub(new_downloaded);

        self.state.current_downloaded = new_downloaded;
        self.state.current_uploaded = new_uploaded;
        self.state.current_left = new_left;

        // Update current ratio
        if self.state.current_downloaded > 0 {
            self.state.current_ratio = self.state.current_uploaded as f64 / self.state.current_downloaded as f64;
        }
    }

    /// Check if target ratio has been reached
    fn check_target_ratio(&self) -> bool {
        if let Some(target) = self.config.target_ratio {
            if self.state.current_downloaded > 0 {
                return self.state.current_ratio >= target;
            }
        }
        false
    }

    /// Rotate peer ID if interval has elapsed
    async fn rotate_peer_if_needed(&mut self) -> Result<()> {
        if let Some(minutes) = self.config.peer_rotation_minutes {
            let interval_secs = minutes * 60;
            if self.last_peer_rotation.elapsed().as_secs() >= interval_secs {
                eprintln!("[Spoofer] Rotating peer ID...");

                // Send "stopped" with old peer_id
                self.announce(Some("stopped")).await.ok();

                // Generate new peer_id and key
                self.peer_id = PeerIdGenerator::generate(self.emulation.get_peer_id_regex())
                    .context("Failed to generate new peer ID")?;
                self.key = KeyGenerator::generate();

                // Reset rotation timer
                self.last_peer_rotation = Instant::now();
                self.state.peer_rotation_count += 1;
                self.state.peer_id = self.peer_id.clone();

                eprintln!("[Spoofer] New peer ID: {} (rotation #{})",
                    self.peer_id, self.state.peer_rotation_count);

                // Send "started" with new peer_id
                self.announce(Some("started")).await?;
            }
        }
        Ok(())
    }

    fn add_to_history(&mut self) {
        let update = ProgressUpdate {
            timestamp: Utc::now(),
            downloaded: self.state.current_downloaded,
            uploaded: self.state.current_uploaded,
            left: self.state.current_left,
            seeders: self.state.seeders,
            leechers: self.state.leechers,
        };

        self.history.push(update.clone());

        // Keep only last 20 entries
        if self.history.len() > 20 {
            self.history.remove(0);
        }

        // Update state history
        self.state.history = self.history.clone();
    }

    pub fn set_stop_signal(&mut self, rx: mpsc::Receiver<()>) {
        self.stop_signal = Some(rx);
    }
}

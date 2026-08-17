export interface ParsedTorrentInfo {
  name: string;
  total_size: number;
  piece_length: number;
  num_trackers: number;
}

export interface SpooferConfig {
  torrent_path: string;
  client_profile: string;
  initial_downloaded: number;
  initial_uploaded: number;
  download_speed: number;
  upload_speed: number;
  port: number;
  // New features
  target_ratio?: number;
  realistic_mode: boolean;
  variance_percent: number;
  peer_rotation_minutes?: number;
}

export type SpooferStatus =
  | "Idle"
  | "Starting"
  | "Running"
  | "Paused"
  | "Stopped"
  | { Error: string };

export interface SpooferState {
  status: SpooferStatus;
  current_downloaded: number;
  current_uploaded: number;
  current_left: number;
  seeders: number;
  leechers: number;
  next_announce_in: number;
  announce_interval: number;
  current_tracker: string;
  torrent_name: string;
  total_size: number;
  history: ProgressUpdate[];
  // New feature states
  current_ratio: number;
  target_ratio?: number;
  peer_rotation_count: number;
  realistic_mode: boolean;
  // Config display
  download_speed: number;
  upload_speed: number;
  variance_percent: number;
  peer_id: string;
  client_profile: string;
  peer_rotation_minutes?: number;
}

export interface ProgressUpdate {
  timestamp: string;
  downloaded: number;
  uploaded: number;
  left: number;
  seeders: number;
  leechers: number;
}

mod parser;

pub use parser::TorrentInfo;

use std::path::Path;
use anyhow::Result;

/// Parse a .torrent file and extract metadata
pub fn parse_torrent_file<P: AsRef<Path>>(path: P) -> Result<TorrentInfo> {
    let bytes = std::fs::read(path)?;
    parser::parse_torrent(&bytes)
}

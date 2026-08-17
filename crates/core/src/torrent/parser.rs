use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentInfo {
    /// Torrent name
    pub name: String,

    /// Total size in bytes
    pub total_size: u64,

    /// Piece size in bytes
    pub piece_length: u64,

    /// URL-encoded SHA1 info hash (for tracker requests)
    pub info_hash: String,

    /// Raw info hash bytes (20 bytes)
    pub info_hash_bytes: Vec<u8>,

    /// List of tracker URLs (HTTP only)
    pub trackers: Vec<String>,
}

/// Parse a torrent file and extract relevant information
pub fn parse_torrent(data: &[u8]) -> Result<TorrentInfo> {
    // Find the info dictionary bounds in raw data
    let (info_start, info_end) = find_info_dict_bounds(data)
        .context("Failed to find info dictionary in torrent")?;

    // Calculate SHA1 hash of the ORIGINAL info bytes
    let info_bytes = &data[info_start..info_end];
    let mut hasher = Sha1::new();
    hasher.update(info_bytes);
    let info_hash_bytes = hasher.finalize().to_vec();

    // URL-encode the info hash (like Go does)
    let info_hash = url_encode_bytes(&info_hash_bytes);

    // Now parse the full torrent to extract metadata
    let decoded = parse_bencode(data, 0)?.0;
    let root = decoded.as_dict().context("Torrent root is not a dictionary")?;

    // Extract trackers
    let mut trackers = Vec::new();

    // Main announce URL
    if let Some(announce) = root.get(b"announce".as_slice()) {
        if let Some(url) = announce.as_string() {
            if url.starts_with("http") {
                trackers.push(url.to_string());
            }
        }
    }

    // Announce list
    if let Some(announce_list) = root.get(b"announce-list".as_slice()) {
        if let Some(tiers) = announce_list.as_list() {
            for tier in tiers {
                if let Some(urls) = tier.as_list() {
                    for url_val in urls {
                        if let Some(url) = url_val.as_string() {
                            if url.starts_with("http") && !trackers.contains(&url.to_string()) {
                                trackers.push(url.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    if trackers.is_empty() {
        return Err(anyhow!("No HTTP trackers found in torrent file"));
    }

    // Extract info dict metadata
    let info = root.get(b"info".as_slice())
        .context("No info dictionary in torrent")?
        .as_dict()
        .context("Info is not a dictionary")?;

    let name = info.get(b"name".as_slice())
        .and_then(|v| v.as_string())
        .context("No name in info dictionary")?
        .to_string();

    let piece_length = info.get(b"piece length".as_slice())
        .and_then(|v| v.as_int())
        .context("No piece length in info dictionary")? as u64;

    // Calculate total size
    let total_size = if let Some(length) = info.get(b"length".as_slice()) {
        // Single file mode
        length.as_int().context("Invalid length")? as u64
    } else if let Some(files) = info.get(b"files".as_slice()) {
        // Multi-file mode
        let files_list = files.as_list().context("Files is not a list")?;
        let mut total = 0u64;
        for file in files_list {
            let file_dict = file.as_dict().context("File entry is not a dict")?;
            let length = file_dict.get(b"length".as_slice())
                .and_then(|v| v.as_int())
                .context("File has no length")?;
            total += length as u64;
        }
        total
    } else {
        return Err(anyhow!("Torrent has neither 'length' nor 'files' field"));
    };

    Ok(TorrentInfo {
        name,
        total_size,
        piece_length,
        info_hash,
        info_hash_bytes,
        trackers,
    })
}

/// URL-encode bytes like the Go implementation does
/// Characters matching [a-zA-Z0-9.\-_~] are kept, others are percent-encoded
fn url_encode_bytes(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &b in bytes {
        if b.is_ascii_alphanumeric()
            || b == b'.'
            || b == b'-'
            || b == b'_'
            || b == b'~'
        {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{:02x}", b));
        }
    }
    result
}

/// Find the byte bounds of the "info" dictionary in the raw torrent data
fn find_info_dict_bounds(data: &[u8]) -> Option<(usize, usize)> {
    // Look for "4:info" followed by a dictionary
    let pattern = b"4:infod";

    for i in 0..data.len().saturating_sub(pattern.len()) {
        if &data[i..i + pattern.len()] == pattern {
            // Found "4:infod", the dict starts at i + 6 (after "4:info")
            let dict_start = i + 6;

            // Find the end of the dictionary
            if let Some(dict_end) = find_dict_end(data, dict_start) {
                return Some((dict_start, dict_end));
            }
        }
    }
    None
}

/// Find the end position of a dictionary starting at `start`
fn find_dict_end(data: &[u8], start: usize) -> Option<usize> {
    if data[start] != b'd' {
        return None;
    }

    let mut pos = start + 1;
    let mut depth = 1;

    while pos < data.len() && depth > 0 {
        match data[pos] {
            b'd' => {
                // Start of nested dictionary
                depth += 1;
                pos += 1;
            }
            b'l' => {
                // Start of list
                depth += 1;
                pos += 1;
            }
            b'e' => {
                // End of dict or list
                depth -= 1;
                pos += 1;
            }
            b'i' => {
                // Integer: i<number>e
                pos += 1;
                while pos < data.len() && data[pos] != b'e' {
                    pos += 1;
                }
                pos += 1; // skip 'e'
            }
            b'0'..=b'9' => {
                // String: <length>:<string>
                let len_start = pos;
                while pos < data.len() && data[pos] != b':' {
                    pos += 1;
                }
                let len_str = std::str::from_utf8(&data[len_start..pos]).ok()?;
                let len: usize = len_str.parse().ok()?;
                pos += 1 + len; // skip ':' and string content
            }
            _ => {
                pos += 1;
            }
        }
    }

    if depth == 0 {
        Some(pos)
    } else {
        None
    }
}

// Simple bencode value type for parsing
#[derive(Debug, Clone)]
enum BencodeValue {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<BencodeValue>),
    Dict(Vec<(Vec<u8>, BencodeValue)>),
}

impl BencodeValue {
    fn as_int(&self) -> Option<i64> {
        match self {
            BencodeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            BencodeValue::Bytes(b) => std::str::from_utf8(b).ok(),
            _ => None,
        }
    }

    fn as_list(&self) -> Option<&Vec<BencodeValue>> {
        match self {
            BencodeValue::List(l) => Some(l),
            _ => None,
        }
    }

    fn as_dict(&self) -> Option<&Vec<(Vec<u8>, BencodeValue)>> {
        match self {
            BencodeValue::Dict(d) => Some(d),
            _ => None,
        }
    }
}

// Extension trait for dict lookup
trait DictExt {
    fn get(&self, key: &[u8]) -> Option<&BencodeValue>;
}

impl DictExt for Vec<(Vec<u8>, BencodeValue)> {
    fn get(&self, key: &[u8]) -> Option<&BencodeValue> {
        self.iter().find(|(k, _)| k.as_slice() == key).map(|(_, v)| v)
    }
}

/// Parse bencode data starting at position
fn parse_bencode(data: &[u8], start: usize) -> Result<(BencodeValue, usize)> {
    if start >= data.len() {
        return Err(anyhow!("Unexpected end of data"));
    }

    match data[start] {
        b'd' => parse_dict(data, start),
        b'l' => parse_list(data, start),
        b'i' => parse_int(data, start),
        b'0'..=b'9' => parse_bytes(data, start),
        c => Err(anyhow!("Unexpected character: {}", c as char)),
    }
}

fn parse_dict(data: &[u8], start: usize) -> Result<(BencodeValue, usize)> {
    let mut pos = start + 1; // skip 'd'
    let mut items = Vec::new();

    while pos < data.len() && data[pos] != b'e' {
        let (key, next) = parse_bytes(data, pos)?;
        pos = next;

        let key_bytes = match key {
            BencodeValue::Bytes(b) => b,
            _ => return Err(anyhow!("Dict key must be bytes")),
        };

        let (value, next) = parse_bencode(data, pos)?;
        pos = next;

        items.push((key_bytes, value));
    }

    Ok((BencodeValue::Dict(items), pos + 1)) // skip 'e'
}

fn parse_list(data: &[u8], start: usize) -> Result<(BencodeValue, usize)> {
    let mut pos = start + 1; // skip 'l'
    let mut items = Vec::new();

    while pos < data.len() && data[pos] != b'e' {
        let (value, next) = parse_bencode(data, pos)?;
        pos = next;
        items.push(value);
    }

    Ok((BencodeValue::List(items), pos + 1)) // skip 'e'
}

fn parse_int(data: &[u8], start: usize) -> Result<(BencodeValue, usize)> {
    let mut pos = start + 1; // skip 'i'
    let int_start = pos;

    while pos < data.len() && data[pos] != b'e' {
        pos += 1;
    }

    let int_str = std::str::from_utf8(&data[int_start..pos])
        .context("Invalid integer string")?;
    let value: i64 = int_str.parse()
        .context("Failed to parse integer")?;

    Ok((BencodeValue::Int(value), pos + 1)) // skip 'e'
}

fn parse_bytes(data: &[u8], start: usize) -> Result<(BencodeValue, usize)> {
    let mut pos = start;

    while pos < data.len() && data[pos] != b':' {
        pos += 1;
    }

    let len_str = std::str::from_utf8(&data[start..pos])
        .context("Invalid length string")?;
    let len: usize = len_str.parse()
        .context("Failed to parse length")?;

    pos += 1; // skip ':'

    let bytes = data[pos..pos + len].to_vec();

    Ok((BencodeValue::Bytes(bytes), pos + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_bytes() {
        let bytes = vec![0x12, 0x34, b'A', b'z', 0xff];
        let encoded = url_encode_bytes(&bytes);
        assert_eq!(encoded, "%124Az%ff");
    }
}

use anyhow::Result;
use rand::Rng;
use rand_regex::Regex;

pub struct PeerIdGenerator;

impl PeerIdGenerator {
    /// Generate a peer ID from a regex pattern
    /// NOTE: Go does NOT URL-encode the peer_id, it's sent raw in the query string
    pub fn generate(regex_pattern: &str) -> Result<String> {
        let regex = Regex::compile(regex_pattern, 100)?;
        let mut rng = rand::thread_rng();
        let peer_id: String = rng.sample(&regex);
        // Return raw peer_id - NOT URL-encoded (like Go)
        Ok(peer_id)
    }
}

pub struct KeyGenerator;

impl KeyGenerator {
    /// Generate a random 8-character hex key
    pub fn generate() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 4] = rng.gen();
        format!("{:02X}{:02X}{:02X}{:02X}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

pub struct ValueRounder {
    piece_length: u64,
}

impl ValueRounder {
    pub fn new(piece_length: u64) -> Self {
        Self { piece_length }
    }

    /// Round values according to qBittorrent behavior
    pub fn round_values(&self, downloaded: u64, uploaded: u64, left: u64) -> (u64, u64, u64) {
        // Downloaded: no rounding (exact value)
        let rounded_downloaded = downloaded;

        // Uploaded: round down to nearest 16 KiB
        let rounded_uploaded = (uploaded / (16 * 1024)) * (16 * 1024);

        // Left: round down to nearest piece size
        let rounded_left = (left / self.piece_length) * self.piece_length;

        (rounded_downloaded, rounded_uploaded, rounded_left)
    }

    /// Add random piece variation for realism
    pub fn add_random_pieces(&self, value: u64, num_pieces_range: (u32, u32)) -> u64 {
        let mut rng = rand::thread_rng();
        let num_pieces = rng.gen_range(num_pieces_range.0..=num_pieces_range.1);
        value + (num_pieces as u64 * self.piece_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = KeyGenerator::generate();
        assert_eq!(key.len(), 8);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit() && c.is_uppercase()));
    }

    #[test]
    fn test_peer_id_generation() {
        let peer_id = PeerIdGenerator::generate("-qB4030-[A-Za-z0-9]{12}").unwrap();
        assert!(peer_id.starts_with("-qB4030-"));
    }

    #[test]
    fn test_value_rounding() {
        let rounder = ValueRounder::new(524288); // 512 KiB pieces

        let (d, u, l) = rounder.round_values(1000000, 1000000, 1000000);

        // Downloaded should be exact
        assert_eq!(d, 1000000);

        // Uploaded should be rounded to 16 KiB
        assert_eq!(u, 983040); // (1000000 / 16384) * 16384

        // Left should be rounded to piece size
        assert_eq!(l, 524288); // (1000000 / 524288) * 524288
    }
}

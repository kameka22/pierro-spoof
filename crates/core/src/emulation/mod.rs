use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientProfile {
    pub name: String,

    #[serde(rename = "peerId")]
    pub peer_id: PeerIdConfig,

    pub key: KeyConfig,

    pub rounding: RoundingConfig,

    pub query: String,

    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerIdConfig {
    pub regex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    pub generator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundingConfig {
    pub generator: String,
}

pub struct ClientEmulation {
    profile: ClientProfile,
}

impl ClientEmulation {
    /// Load a client profile by name
    pub fn load(client_name: &str) -> Result<Self> {
        let profile = match client_name {
            "qbit-4.0.3" => {
                let json = include_str!("../../profiles/qbit-4.0.3.json");
                serde_json::from_str(json)?
            }
            "qbit-4.3.3" => {
                let json = include_str!("../../profiles/qbit-4.3.3.json");
                serde_json::from_str(json)?
            }
            _ => return Err(anyhow!("Unknown client profile: {}", client_name)),
        };

        Ok(Self { profile })
    }

    pub fn profile(&self) -> &ClientProfile {
        &self.profile
    }

    pub fn get_query_template(&self) -> &str {
        &self.profile.query
    }

    pub fn get_headers(&self) -> &HashMap<String, String> {
        &self.profile.headers
    }

    pub fn get_peer_id_regex(&self) -> &str {
        &self.profile.peer_id.regex
    }

    /// List available client profiles
    pub fn available_profiles() -> Vec<(&'static str, &'static str)> {
        vec![
            ("qbit-4.0.3", "qBittorrent v4.0.3"),
            ("qbit-4.3.3", "qBittorrent v4.3.3"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_profiles() {
        let emulation = ClientEmulation::load("qbit-4.0.3").unwrap();
        assert_eq!(emulation.profile().name, "qBittorrent v4.0.3");

        let emulation = ClientEmulation::load("qbit-4.3.3").unwrap();
        assert_eq!(emulation.profile().name, "qBittorrent v4.3.3");
    }
}

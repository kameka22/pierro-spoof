use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use reqwest::dns::{Resolve, Resolving, Name};
use hickory_resolver::config::{ResolverConfig, NameServerConfig};
use hickory_resolver::Resolver;
use hickory_net::runtime::TokioRuntimeProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceRequest {
    pub info_hash: String,
    pub peer_id: String,
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub key: String,
    pub event: Option<String>, // "started", "stopped", "completed", or None
    pub numwant: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceResponse {
    pub interval: u64,
    pub complete: u32,    // seeders
    pub incomplete: u32,  // leechers
    pub failure_reason: Option<String>,
}

pub struct TrackerClient {
    trackers: Vec<String>,
    current_tracker_index: usize,
    client: reqwest::Client,
}

/// Custom DNS resolver that uses Google DNS (8.8.8.8) via hickory-dns
/// This bypasses local DNS that might block tracker domains
struct GoogleDnsResolver {
    resolver: Resolver<TokioRuntimeProvider>,
}

impl GoogleDnsResolver {
    fn new() -> Self {
        // Configure to use Google DNS (8.8.8.8)
        let google_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let name_server = NameServerConfig::udp_and_tcp(google_ip);
        let config = ResolverConfig::from_parts(
            None,
            vec![],
            vec![name_server],
        );
        let resolver = Resolver::builder_with_config(config, TokioRuntimeProvider::default())
            .build()
            .expect("Failed to create DNS resolver");
        Self { resolver }
    }
}

impl Resolve for GoogleDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.resolver.clone();
        Box::pin(async move {
            let lookup = resolver.lookup_ip(name.as_str()).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let addrs: Vec<SocketAddr> = lookup
                .iter()
                .map(|ip| SocketAddr::new(ip, 0))
                .collect();

            Ok(Box::new(addrs.into_iter()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
        })
    }
}

impl TrackerClient {
    pub fn new(trackers: Vec<String>) -> Self {
        // Match Go's http.DefaultClient behavior more closely
        // Go's default client has no timeout and uses system TLS
        let client = reqwest::Client::builder()
            // No timeout like Go's default client
            .gzip(true)
            // Don't require valid certs for some trackers
            .danger_accept_invalid_certs(true)
            // Use Google DNS (8.8.8.8) to bypass local DNS blocking
            .dns_resolver(Arc::new(GoogleDnsResolver::new()))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            trackers,
            current_tracker_index: 0,
            client,
        }
    }

    pub fn get_current_tracker(&self) -> &str {
        &self.trackers[self.current_tracker_index]
    }

    /// Swap successful tracker to front (like Go)
    fn swap_to_front(&mut self, idx: usize) {
        if idx != 0 && idx < self.trackers.len() {
            self.trackers.swap(0, idx);
            self.current_tracker_index = 0;
        }
    }

    pub async fn announce(
        &mut self,
        request: &AnnounceRequest,
        query_template: &str,
        headers: &HashMap<String, String>,
        retry: bool,
    ) -> Result<AnnounceResponse> {
        if retry {
            // Infinite retry with exponential backoff like Go
            let mut retry_delay: u64 = 30;

            loop {
                match self.try_announce_all_trackers(request, query_template, headers).await {
                    Ok(response) => {
                        return Ok(response);
                    }
                    Err(e) => {
                        eprintln!("[Tracker] Announce failed: {}. Retrying in {}s...", e, retry_delay);
                        tokio::time::sleep(Duration::from_secs(retry_delay)).await;

                        // Exponential backoff, cap at 900 seconds (like Go)
                        retry_delay *= 2;
                        if retry_delay > 900 {
                            retry_delay = 900;
                        }
                    }
                }
            }
        } else {
            // No retry - fail immediately
            self.try_announce_all_trackers(request, query_template, headers).await
        }
    }

    /// Try all trackers once (like Go's tryMakeRequest loop)
    async fn try_announce_all_trackers(
        &mut self,
        request: &AnnounceRequest,
        query_template: &str,
        headers: &HashMap<String, String>,
    ) -> Result<AnnounceResponse> {
        for idx in 0..self.trackers.len() {
            let tracker_idx = (self.current_tracker_index + idx) % self.trackers.len();

            match self.try_announce_single(tracker_idx, request, query_template, headers).await {
                Ok(response) => {
                    // Swap successful tracker to front (like Go)
                    self.swap_to_front(tracker_idx);
                    return Ok(response);
                }
                Err(e) => {
                    eprintln!("[Tracker] Tracker {} failed: {}", self.trackers[tracker_idx], e);
                    continue;
                }
            }
        }

        Err(anyhow!("All trackers failed"))
    }

    async fn try_announce_single(
        &self,
        tracker_idx: usize,
        request: &AnnounceRequest,
        query_template: &str,
        headers: &HashMap<String, String>,
    ) -> Result<AnnounceResponse> {
        let tracker_url = &self.trackers[tracker_idx];

        // Build query string from template (use same placeholders as Go)
        let query = query_template
            .replace("{infohash}", &request.info_hash)
            .replace("{peerid}", &request.peer_id)
            .replace("{port}", &request.port.to_string())
            .replace("{uploaded}", &request.uploaded.to_string())
            .replace("{downloaded}", &request.downloaded.to_string())
            .replace("{left}", &request.left.to_string())
            .replace("{key}", &request.key)
            .replace("{numwant}", &request.numwant.to_string())
            .replace("{event}", request.event.as_deref().unwrap_or(""));

        // Build URL like Go does - handle case where tracker already has query params
        let url = if tracker_url.contains('?') {
            format!("{}&{}", tracker_url, query.trim_start_matches('&'))
        } else {
            format!("{}?{}", tracker_url, query.trim_start_matches('?'))
        };

        eprintln!("[Tracker] Request URL: {}", url);

        // Build request with custom headers
        let mut req = self.client.get(&url);
        for (key, value) in headers {
            req = req.header(key, value);
        }

        // Send request
        let response = match req.send().await {
            Ok(r) => {
                eprintln!("[Tracker] Got response with status: {}", r.status());
                r
            }
            Err(e) => {
                eprintln!("[Tracker] Request failed: {}", e);
                if e.is_timeout() {
                    eprintln!("[Tracker] Error type: TIMEOUT");
                } else if e.is_connect() {
                    eprintln!("[Tracker] Error type: CONNECTION");
                } else if e.is_request() {
                    eprintln!("[Tracker] Error type: REQUEST BUILD");
                }
                if let Some(source) = e.source() {
                    eprintln!("[Tracker] Error source: {}", source);
                }
                return Err(anyhow!("Failed to send announce request: {}", e));
            }
        };

        let status = response.status();

        // Check status code (Go only accepts 200 OK)
        if status != reqwest::StatusCode::OK {
            return Err(anyhow!("Tracker returned status: {}", status));
        }

        let body = response.bytes().await
            .context("Failed to read response body")?;

        // Skip empty responses (like Go)
        if body.is_empty() {
            return Err(anyhow!("Empty response from tracker"));
        }

        eprintln!("[Tracker] Response body length: {} bytes", body.len());

        // Parse bencode response
        let (interval, complete, incomplete, failure_reason) = parse_tracker_response(&body)?;

        if let Some(reason) = failure_reason {
            return Err(anyhow!("Tracker error: {}", reason));
        }

        eprintln!("[Tracker] Parsed: interval={}, seeders={}, leechers={}", interval, complete, incomplete);

        Ok(AnnounceResponse {
            interval,
            complete,
            incomplete,
            failure_reason: None,
        })
    }
}

/// Parse tracker response bencode manually (like Go's extractTrackerResponse)
fn parse_tracker_response(data: &[u8]) -> Result<(u64, u32, u32, Option<String>)> {
    let mut interval: u64 = 1800;
    let mut complete: u32 = 0;
    let mut incomplete: u32 = 0;
    let mut failure_reason: Option<String> = None;

    // Simple bencode dict parser
    if data.is_empty() || data[0] != b'd' {
        return Err(anyhow!("Invalid tracker response: not a dictionary"));
    }

    let mut pos = 1; // skip 'd'

    while pos < data.len() && data[pos] != b'e' {
        // Read key (string)
        let (key, next_pos) = read_bencode_string(data, pos)?;
        pos = next_pos;

        // Read value based on key
        match key.as_slice() {
            b"interval" => {
                if let Ok((val, next)) = read_bencode_int(data, pos) {
                    interval = val as u64;
                    pos = next;
                } else {
                    pos = skip_bencode_value(data, pos)?;
                }
            }
            b"min interval" => {
                // Skip min interval, use interval only
                pos = skip_bencode_value(data, pos)?;
            }
            b"complete" => {
                if let Ok((val, next)) = read_bencode_int(data, pos) {
                    complete = val as u32;
                    pos = next;
                } else {
                    pos = skip_bencode_value(data, pos)?;
                }
            }
            b"incomplete" => {
                if let Ok((val, next)) = read_bencode_int(data, pos) {
                    incomplete = val as u32;
                    pos = next;
                } else {
                    pos = skip_bencode_value(data, pos)?;
                }
            }
            b"failure reason" => {
                if let Ok((val, next)) = read_bencode_string(data, pos) {
                    failure_reason = Some(String::from_utf8_lossy(&val).to_string());
                    pos = next;
                } else {
                    pos = skip_bencode_value(data, pos)?;
                }
            }
            _ => {
                // Skip unknown values
                pos = skip_bencode_value(data, pos)?;
            }
        }
    }

    Ok((interval, complete, incomplete, failure_reason))
}

fn read_bencode_string(data: &[u8], start: usize) -> Result<(Vec<u8>, usize)> {
    let mut pos = start;

    // Read length
    while pos < data.len() && data[pos] != b':' {
        if !data[pos].is_ascii_digit() {
            return Err(anyhow!("Invalid string length at pos {}", pos));
        }
        pos += 1;
    }

    if pos >= data.len() {
        return Err(anyhow!("Unexpected end of data while reading string length"));
    }

    let len_str = std::str::from_utf8(&data[start..pos])
        .context("Invalid length encoding")?;
    let len: usize = len_str.parse()
        .context("Failed to parse string length")?;

    pos += 1; // skip ':'

    if pos + len > data.len() {
        return Err(anyhow!("String extends beyond data"));
    }

    let value = data[pos..pos + len].to_vec();
    Ok((value, pos + len))
}

fn read_bencode_int(data: &[u8], start: usize) -> Result<(i64, usize)> {
    if start >= data.len() || data[start] != b'i' {
        return Err(anyhow!("Expected integer at pos {}", start));
    }

    let mut pos = start + 1;
    let int_start = pos;

    while pos < data.len() && data[pos] != b'e' {
        pos += 1;
    }

    if pos >= data.len() {
        return Err(anyhow!("Unexpected end of data while reading integer"));
    }

    let int_str = std::str::from_utf8(&data[int_start..pos])
        .context("Invalid integer encoding")?;
    let value: i64 = int_str.parse()
        .context("Failed to parse integer")?;

    Ok((value, pos + 1)) // skip 'e'
}

fn skip_bencode_value(data: &[u8], start: usize) -> Result<usize> {
    if start >= data.len() {
        return Err(anyhow!("Unexpected end of data"));
    }

    match data[start] {
        b'i' => {
            // Integer: find 'e'
            let mut pos = start + 1;
            while pos < data.len() && data[pos] != b'e' {
                pos += 1;
            }
            Ok(pos + 1)
        }
        b'l' => {
            // List
            let mut pos = start + 1;
            while pos < data.len() && data[pos] != b'e' {
                pos = skip_bencode_value(data, pos)?;
            }
            Ok(pos + 1)
        }
        b'd' => {
            // Dict
            let mut pos = start + 1;
            while pos < data.len() && data[pos] != b'e' {
                // Skip key
                let (_, next) = read_bencode_string(data, pos)?;
                pos = next;
                // Skip value
                pos = skip_bencode_value(data, pos)?;
            }
            Ok(pos + 1)
        }
        b'0'..=b'9' => {
            // String
            let (_, next) = read_bencode_string(data, start)?;
            Ok(next)
        }
        _ => Err(anyhow!("Unknown bencode type at pos {}: {:02x}", start, data[start])),
    }
}

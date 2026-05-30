//! Discovers a running League Client and builds an authenticated HTTPS client.
//!
//! The League Client (LeagueClientUx.exe) exposes a local REST API (the "LCU
//! API") on a random port secured with a self-signed certificate and HTTP basic
//! auth. Both the port and the auth token are passed to the client process as
//! command-line arguments (`--app-port` and `--remoting-auth-token`), so we can
//! recover them by inspecting the running process — no lockfile path required.

use base64::Engine;
use sysinfo::System;

/// An authenticated handle to the local League Client API.
pub struct Connection {
    pub port: u16,
    pub client: reqwest::Client,
    auth_header: String,
}

impl Connection {
    pub fn new(port: u16, token: &str) -> reqwest::Result<Self> {
        // The LCU serves a self-signed cert on 127.0.0.1; accepting invalid
        // certs is safe here because we only ever talk to loopback.
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("riot:{token}"));
        Ok(Self {
            port,
            client,
            auth_header: format!("Basic {encoded}"),
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{}", self.port, path)
    }

    /// A request builder with the auth header already applied.
    pub fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, self.url(path))
            .header("Authorization", &self.auth_header)
    }
}

/// Connection details for both local servers, recovered from the League
/// Client's command line. The LeagueClientUx process is launched with the LCU
/// port/token (`--app-port` / `--remoting-auth-token`) *and* the Riot Client's
/// port/token (`--riotclient-app-port` / `--riotclient-auth-token`).
pub struct Discovered {
    /// League Client (LeagueClientUx) — serves the `/lol-...` endpoints.
    pub lcu: (u16, String),
    /// Riot Client (RiotClientServices) — serves `/chat/v5/...`, `/riotclient/...`.
    pub riot: Option<(u16, String)>,
}

/// Scan running processes for the League Client and extract both servers'
/// (port, token). Returns `None` if the client isn't running.
pub fn discover() -> Option<Discovered> {
    let sys = System::new_all();
    for proc in sys.processes().values() {
        let name = proc.name().to_string_lossy();
        if !name.eq_ignore_ascii_case("LeagueClientUx.exe") {
            continue;
        }
        let (mut lcu_port, mut lcu_token) = (None, None);
        let (mut riot_port, mut riot_token) = (None, None);
        for arg in proc.cmd() {
            let arg = arg.to_string_lossy();
            if let Some(v) = arg.strip_prefix("--app-port=") {
                lcu_port = v.trim_matches('"').parse().ok();
            } else if let Some(v) = arg.strip_prefix("--remoting-auth-token=") {
                lcu_token = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = arg.strip_prefix("--riotclient-app-port=") {
                riot_port = v.trim_matches('"').parse().ok();
            } else if let Some(v) = arg.strip_prefix("--riotclient-auth-token=") {
                riot_token = Some(v.trim_matches('"').to_string());
            }
        }
        if let (Some(p), Some(t)) = (lcu_port, lcu_token) {
            let riot = match (riot_port, riot_token) {
                (Some(rp), Some(rt)) => Some((rp, rt)),
                _ => None,
            };
            return Some(Discovered { lcu: (p, t), riot });
        }
    }
    None
}

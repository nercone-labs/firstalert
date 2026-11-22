//! Messages of the P2P Quake JSON API v2 (P2P地震情報, <https://www.p2pquake.net/develop/json_api_v2/>,
//! specification `json-api-v2.yaml` of <https://github.com/p2pquake/epsp-specifications>).
//!
//! The WebSocket API (`wss://api.p2pquake.net/v2/ws`) sends one JSON object per text frame, and
//! may send the same one more than once (its `id` tells them apart); the HTTP API `/history`
//! sends arrays of the same objects. Each object names its kind by `code`. Fields are read
//! leniently (`lenient`).

mod earthquake;
mod eew;
mod message;
mod tsunami;

pub use earthquake::{DomesticTsunami, Earthquake, IssueKind, ObservationPoint};
pub use eew::{Eew, EewArea};
pub use message::{Message, ParseError};
pub use tsunami::{Tsunami, TsunamiArea};

/// The URLs of the API.
pub struct Api;

impl Api {
    pub const WEBSOCKET: &'static str = "wss://api.p2pquake.net/v2/ws";
    pub const HISTORY: &'static str = "https://api.p2pquake.net/v2/history";

    /// The URL of the latest `limit` (1 to 100) messages of the given codes, newest first.
    pub fn history_url(codes: &[u32], limit: u32) -> String {
        let codes: Vec<String> = codes.iter().map(|c| format!("codes={c}")).collect();
        format!("{}?{}&limit={}", Self::HISTORY, codes.join("&"), limit.clamp(1, 100))
    }
}

/// Reads a time of the API: JST, `2006/01/02 15:04:05.999`.
pub(crate) fn time(text: Option<String>) -> Option<jma::Time> {
    text.and_then(|t| jma::Time::parse_jst(&t))
}

/// Reads a hypocentre: `-200` for unknown coordinates, `-1` for unknown depth and magnitude.
pub(crate) fn hypocentre(value: &serde_json::Value, path: &[&str]) -> (Option<geodesy::Coordinate>, Option<f64>, Option<f64>) {
    use lenient::Lenient;
    let field = |key: &str| {
        let mut full = path.to_vec();
        full.push(key);
        value.number(&full)
    };
    let epicentre = match (field("latitude"), field("longitude")) {
        (Some(lat), Some(lon)) if lat > -200.0 && lon > -200.0 => geodesy::Coordinate::checked(lat, lon),
        _ => None,
    };
    // Only the whole part of the depth is valid (a decimal point is added by the system).
    let depth = field("depth").filter(|d| *d >= 0.0).map(f64::trunc);
    let magnitude = field("magnitude").filter(|m| *m >= 0.0);
    (epicentre, depth, magnitude)
}

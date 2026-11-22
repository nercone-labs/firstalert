//! What the platform and the core say to each other. Every type here is serialised as JSON with
//! a `type` tag, which is how the C interface carries it.

use crate::alert::{AlertLevel, Notification};
use crate::connection::{ConnectionState, SourceId};
use crate::event::{EarthquakeEvent, EventDetail};
use crate::settings::{Language, Place, Settings};
use crate::tsunami::{SpecialEntry, TsunamiState};
use crate::words::IntensityLabel;
use geodesy::Coordinate;
use jma::Time;
use serde::{Deserialize, Serialize};

/// An input of the core. Every input carries the time of the platform (`now`, ms since the
/// epoch), the only clock the core knows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    /// The first input: where the core keeps its files (`None` keeps nothing), and the locale
    /// of the system. The core loads its settings and history, and starts receiving.
    Start { now: Time, data_dir: Option<String>, locale: String },
    /// The connection to `source` is open.
    Opened { now: Time, source: SourceId },
    /// A text message from `source`.
    Received { now: Time, source: SourceId, text: String },
    /// The connection to `source` closed or failed; `reason` is for the log.
    Closed { now: Time, source: SourceId, reason: String },
    /// The answer to `Effect::Request`: the HTTP status and the body, or `status` 0 and the
    /// error in `body` when there was no answer.
    Responded { now: Time, id: u64, status: u16, body: String },
    /// Time passed: the platform sends this at the time of `Effect::Wake`, or whenever it likes.
    Tick { now: Time },
    /// The network came back (or changed): reconnect now rather than after the back-off.
    NetworkAvailable { now: Time },
    /// Replace the settings.
    Configure { now: Time, settings: Settings },
    /// The location of the device, when the place follows it.
    Located { now: Time, coordinate: Coordinate },
    /// The locale of the system changed.
    Locale { now: Time, locale: String },
    /// The current state, for a screen.
    Snapshot { now: Time },
    /// Everything known of one earthquake, by its key.
    Detail { now: Time, key: String },
    /// The radii of the P and S waves of the active early warnings, for a map.
    Waves { now: Time },
    /// The outlines of the prefectures, for a map.
    Map { now: Time },
    /// The areas of the warnings (細分区域) of a prefecture, for choosing the user's area, and
    /// the representative point of the prefecture.
    Areas { now: Time, prefecture: u8 },
    /// The user saw the alert of `key`: stop its sound and voice.
    Acknowledge { now: Time, key: String },
    /// Shows a sample alert at `level`, to try the sound, voice and screen.
    Test { now: Time, level: AlertLevel },
    /// Stop receiving (the platform closes the connections).
    Stop { now: Time },
}

impl Input {
    pub fn now(&self) -> Time {
        match self {
            Input::Start { now, .. }
            | Input::Opened { now, .. }
            | Input::Received { now, .. }
            | Input::Closed { now, .. }
            | Input::Responded { now, .. }
            | Input::Tick { now }
            | Input::NetworkAvailable { now }
            | Input::Configure { now, .. }
            | Input::Located { now, .. }
            | Input::Locale { now, .. }
            | Input::Snapshot { now }
            | Input::Detail { now, .. }
            | Input::Waves { now }
            | Input::Map { now }
            | Input::Areas { now, .. }
            | Input::Acknowledge { now, .. }
            | Input::Test { now, .. }
            | Input::Stop { now } => *now,
        }
    }
}

/// Something the platform is to do, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// Open a WebSocket to `url` for `source`, replacing any connection it has. When
    /// `ping_interval_ms` is given, send WebSocket pings at that interval and close the
    /// connection when they go unanswered.
    Connect { source: SourceId, url: String, ping_interval_ms: Option<u64> },
    /// Send a text message on the connection of `source`.
    Send { source: SourceId, text: String },
    /// Close the connection of `source`.
    Disconnect { source: SourceId },
    /// Fetch `url` with GET and answer with `Input::Responded` of the same `id`.
    Request { id: u64, url: String },
    /// Show (or update, by its key) a notification.
    Notify { notification: Box<Notification> },
    /// Remove the notification of `key`.
    Withdraw { key: String },
    /// Stop the sound, voice, vibration and flash of the alert of `key`.
    Silence { key: String },
    /// Send `Input::Tick` at `at`, or earlier. A later `Wake` replaces an earlier one.
    Wake { at: Time },
    /// A line for the log of the platform.
    Log { level: LogLevel, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

/// The answer to every input.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Response {
    pub effects: Vec<Effect>,
    /// The state, when it changed (or when asked for with `Input::Snapshot`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Box<Snapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<EventDetail>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waves: Option<Vec<Waves>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<Vec<MapShape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub areas: Option<PrefectureAreas>,
    /// The input could not be handled (such as JSON that is not an input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The state of the core, for the screens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub now: Time,
    /// The difference of the servers' clock from the platform's, ms (positive when the platform
    /// is behind). Countdowns add it to the platform's clock.
    pub clock_offset_ms: i64,
    pub language: Language,
    pub settings: Settings,
    pub connections: Vec<ConnectionStatus>,
    /// Earthquakes, newest first, without their observation points (see `Input::Detail`).
    pub earthquakes: Vec<EarthquakeEvent>,
    pub tsunami: Option<TsunamiState>,
    pub special: Vec<SpecialEntry>,
    /// The keys of what is happening now, most severe first: early warnings within minutes of
    /// their last report, and tsunami warnings in force.
    pub active: Vec<String>,
    /// The highest level of what is active.
    pub level: AlertLevel,
    pub place: Option<PlaceSummary>,
    /// The names of the intensity classes and what they feel like, for legends.
    pub intensities: Vec<IntensityLabel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub source: SourceId,
    pub enabled: bool,
    pub state: ConnectionState,
    /// Since when the state is what it is.
    pub since: Option<Time>,
    /// When the last message (heartbeats included) came.
    pub last_message: Option<Time>,
    pub failures: u32,
}

/// The user's place in words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceSummary {
    pub place: Place,
    /// The name to show: the area, or the prefecture.
    pub label: String,
}

/// The circles of the waves of one early warning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waves {
    pub key: String,
    pub epicentre: Coordinate,
    pub p_radius_km: Option<f64>,
    pub s_radius_km: Option<f64>,
}

/// The areas of a prefecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefectureAreas {
    pub prefecture: u8,
    pub name: String,
    pub representative: Option<Coordinate>,
    pub areas: Vec<String>,
}

/// The outline of a prefecture: rings of `[longitude, latitude]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapShape {
    pub prefecture: u8,
    pub name: String,
    pub rings: Vec<Vec<[f64; 2]>>,
}

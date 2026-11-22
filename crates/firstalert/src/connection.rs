//! The connections to the relays: when to connect, when to send a ping, when a connection has
//! gone silent, and how long to wait before trying again.

use crate::protocol::Effect;
use jma::Time;
use serde::{Deserialize, Serialize};

/// A relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceId {
    Wolfx,
    P2pquake,
}

impl SourceId {
    pub const ALL: [SourceId; 2] = [SourceId::Wolfx, SourceId::P2pquake];

    pub fn url(&self) -> String {
        match self {
            SourceId::Wolfx => wolfx::Endpoint::combined_websocket_url(),
            SourceId::P2pquake => p2pquake::Api::WEBSOCKET.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    /// Turned off in the settings.
    Disabled,
    /// Waiting to try again after a failure.
    Waiting,
    Connecting,
    Open,
}

/// One connection and its timers.
#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub source: SourceId,
    pub enabled: bool,
    pub state: ConnectionState,
    pub since: Option<Time>,
    pub last_message: Option<Time>,
    /// Failures in a row, for the back-off.
    pub failures: u32,
    retry_at: Option<Time>,
    last_ping: Option<Time>,
}

impl Connection {
    /// How long a connection may take to open.
    pub const CONNECT_TIMEOUT_MS: i64 = 15_000;
    /// The interval of the WebSocket pings the platform sends.
    pub const PING_INTERVAL_MS: u64 = 25_000;
    /// The interval of Wolfx's own `ping`, which its server answers with `pong`.
    pub const WOLFX_PING_MS: i64 = 30_000;
    /// Wolfx sends a heartbeat every minute and answers pings: this long without any message
    /// means the connection is dead even if the socket is not closed.
    pub const WOLFX_SILENCE_MS: i64 = 100_000;
    /// A connection open this long has recovered: the back-off starts again from the shortest.
    pub const STABLE_MS: i64 = 60_000;
    /// The waits before trying again, by failures in a row; the last repeats.
    pub const BACKOFF_MS: [i64; 7] = [500, 1_000, 2_000, 5_000, 10_000, 20_000, 30_000];

    pub fn new(source: SourceId, enabled: bool) -> Connection {
        Connection {
            source,
            enabled,
            state: if enabled { ConnectionState::Waiting } else { ConnectionState::Disabled },
            since: None,
            last_message: None,
            failures: 0,
            retry_at: None,
            last_ping: None,
        }
    }

    fn connect(&mut self, now: Time) -> Effect {
        self.state = ConnectionState::Connecting;
        self.since = Some(now);
        self.retry_at = None;
        self.last_ping = None;
        Effect::Connect { source: self.source, url: self.source.url(), ping_interval_ms: Some(Self::PING_INTERVAL_MS) }
    }

    /// Starts connecting now, unless already connecting or open or turned off.
    pub fn start(&mut self, now: Time) -> Vec<Effect> {
        match self.state {
            ConnectionState::Waiting => vec![self.connect(now)],
            _ => vec![],
        }
    }

    /// Turns the connection on or off.
    pub fn set_enabled(&mut self, enabled: bool, now: Time) -> Vec<Effect> {
        if enabled == self.enabled {
            return vec![];
        }
        self.enabled = enabled;
        if enabled {
            self.state = ConnectionState::Waiting;
            self.failures = 0;
            vec![self.connect(now)]
        } else {
            let was_connected = matches!(self.state, ConnectionState::Open | ConnectionState::Connecting);
            self.state = ConnectionState::Disabled;
            self.since = Some(now);
            self.retry_at = None;
            if was_connected { vec![Effect::Disconnect { source: self.source }] } else { vec![] }
        }
    }

    pub fn opened(&mut self, now: Time) -> Vec<Effect> {
        if !self.enabled {
            return vec![Effect::Disconnect { source: self.source }];
        }
        self.state = ConnectionState::Open;
        self.since = Some(now);
        self.last_message = Some(now);
        vec![]
    }

    pub fn received(&mut self, now: Time) {
        self.last_message = Some(now);
    }

    /// The connection closed or failed: try again after the back-off.
    pub fn closed(&mut self, now: Time) {
        if !self.enabled {
            self.state = ConnectionState::Disabled;
            return;
        }
        let wait = Self::BACKOFF_MS[(self.failures as usize).min(Self::BACKOFF_MS.len() - 1)];
        self.failures = self.failures.saturating_add(1);
        self.state = ConnectionState::Waiting;
        self.since = Some(now);
        self.retry_at = Some(Time(now.0 + wait));
    }

    /// The network changed: a connection that was waiting tries at once, and one that is open
    /// over the old network is replaced (its socket may be dead without knowing it yet). A
    /// connection that opened a moment ago is already on the new network.
    pub fn network_available(&mut self, now: Time) -> Vec<Effect> {
        match self.state {
            ConnectionState::Waiting => vec![self.connect(now)],
            ConnectionState::Open if self.since.is_some_and(|s| now.0 - s.0 > 5_000) => vec![Effect::Disconnect { source: self.source }, self.connect(now)],
            _ => vec![],
        }
    }

    /// What is due at `now`.
    pub fn tick(&mut self, now: Time) -> Vec<Effect> {
        match self.state {
            ConnectionState::Waiting if self.retry_at.is_none_or(|at| now >= at) => vec![self.connect(now)],
            ConnectionState::Connecting if self.since.is_some_and(|s| now.0 - s.0 >= Self::CONNECT_TIMEOUT_MS) => {
                self.closed(now);
                vec![
                    Effect::Disconnect { source: self.source },
                    Effect::Log { level: crate::protocol::LogLevel::Warning, message: format!("{:?}: no connection in time", self.source) },
                ]
            }
            ConnectionState::Open => {
                let mut effects = Vec::new();
                if self.failures > 0 && self.since.is_some_and(|s| now.0 - s.0 >= Self::STABLE_MS) {
                    self.failures = 0;
                }
                if self.source == SourceId::Wolfx {
                    if self.last_message.is_some_and(|m| now.0 - m.0 >= Self::WOLFX_SILENCE_MS) {
                        self.closed(now);
                        effects.push(Effect::Disconnect { source: self.source });
                        effects.push(Effect::Log { level: crate::protocol::LogLevel::Warning, message: "Wolfx: silent, reconnecting".into() });
                        // Waiting from here; reconnect at once, the silence was the failure.
                        self.retry_at = Some(now);
                        effects.push(self.connect(now));
                        return effects;
                    }
                    if self.last_ping.is_none_or(|p| now.0 - p.0 >= Self::WOLFX_PING_MS) {
                        self.last_ping = Some(now);
                        effects.push(Effect::Send { source: self.source, text: wolfx::Message::PING.to_owned() });
                    }
                }
                effects
            }
            _ => vec![],
        }
    }

    /// When `tick` next has something to do.
    pub fn next_wake(&self) -> Option<Time> {
        match self.state {
            ConnectionState::Waiting => self.retry_at,
            ConnectionState::Connecting => self.since.map(|s| Time(s.0 + Self::CONNECT_TIMEOUT_MS)),
            ConnectionState::Open if self.source == SourceId::Wolfx => {
                let ping = self.last_ping.map(|p| p.0 + Self::WOLFX_PING_MS);
                let silence = self.last_message.map(|m| m.0 + Self::WOLFX_SILENCE_MS);
                [ping, silence].into_iter().flatten().min().map(Time)
            }
            ConnectionState::Open if self.failures > 0 => self.since.map(|s| Time(s.0 + Self::STABLE_MS)),
            _ => None,
        }
    }
}

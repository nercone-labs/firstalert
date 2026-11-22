use crate::{EarthquakeList, Eew, SpecialReports, Tsunami};
use lenient::Lenient;
use serde_json::Value;
use std::fmt;

/// One message of the WebSocket API.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Sent every minute and after connecting, to keep the connection open.
    Heartbeat {
        id: Option<String>,
        timestamp: Option<i64>,
    },
    /// The answer to the text `ping`.
    Pong {
        timestamp: Option<i64>,
    },
    Eew(Box<Eew>),
    EarthquakeList(Box<EarthquakeList>),
    Tsunami(Box<Tsunami>),
    Special(Box<SpecialReports>),
    /// A message of another source (such as `cenc_eew`), by its `type`.
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The text is not a JSON object.
    Json(String),
    /// The message has no `type`.
    NoType,
    /// The message has the `type` of a source, but lacks what identifies its content.
    Incomplete(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Json(e) => write!(f, "not JSON: {e}"),
            ParseError::NoType => f.write_str("no type"),
            ParseError::Incomplete(what) => write!(f, "incomplete {what}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// The sources this crate reads, with their names in the paths and in `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    JmaEew,
    JmaEarthquakeList,
    JmaTsunami,
    JmaSpecial,
}

impl Endpoint {
    pub const WEBSOCKET_BASE: &'static str = "wss://ws-api.wolfx.jp";
    pub const HTTP_BASE: &'static str = "https://api.wolfx.jp";

    /// Every source `all_eew` can send, as of 2026-11. Excluding the ones that are not read keeps
    /// the connection quiet; a name the server does not know fails the handshake (HTTP 400), so
    /// only names it documents are listed.
    pub const OTHER_SOURCES: [&'static str; 5] = ["sc_eew", "fj_eew", "cq_eew", "cenc_eew", "cenc_eqlist"];

    pub fn name(&self) -> &'static str {
        match self {
            Endpoint::JmaEew => "jma_eew",
            Endpoint::JmaEarthquakeList => "jma_eqlist",
            Endpoint::JmaTsunami => "jma_tsunami",
            Endpoint::JmaSpecial => "jma_special_eq",
        }
    }

    /// The text that asks a WebSocket connection for the cached JSON of the source.
    pub fn query(&self) -> &'static str {
        match self {
            Endpoint::JmaEew => "query_jmaeew",
            Endpoint::JmaEarthquakeList => "query_jmaeqlist",
            Endpoint::JmaTsunami => "query_jmatsunami",
            Endpoint::JmaSpecial => "query_jmaspecialeq",
        }
    }

    pub fn http_url(&self) -> String {
        format!("{}/{}.json", Self::HTTP_BASE, self.name())
    }

    /// The URL of the `all_eew` channel with every source excluded but these.
    pub fn combined_websocket_url() -> String {
        format!("{}/all_eew?exclude={}", Self::WEBSOCKET_BASE, Self::OTHER_SOURCES.join(","))
    }
}

impl Message {
    /// The text a client sends to keep the connection alive and see that it is.
    pub const PING: &'static str = "ping";

    /// Reads one WebSocket message.
    pub fn parse(text: &str) -> Result<Message, ParseError> {
        let value: Value = serde_json::from_str(text).map_err(|e| ParseError::Json(e.to_string()))?;
        if !value.is_object() {
            return Err(ParseError::Json("not an object".into()));
        }
        let kind = value.text(&["type"]).ok_or(ParseError::NoType)?;
        Self::from_value(&kind, &value)
    }

    /// Reads the JSON of the HTTP API of `endpoint`, which has no `type`.
    pub fn parse_http(endpoint: Endpoint, text: &str) -> Result<Message, ParseError> {
        let value: Value = serde_json::from_str(text).map_err(|e| ParseError::Json(e.to_string()))?;
        Self::from_value(endpoint.name(), &value)
    }

    fn from_value(kind: &str, value: &Value) -> Result<Message, ParseError> {
        Ok(match kind {
            "heartbeat" => Message::Heartbeat { id: value.text(&["id"]), timestamp: value.integer(&["timestamp"]) },
            "pong" => Message::Pong { timestamp: value.integer(&["timestamp"]) },
            "jma_eew" => Message::Eew(Box::new(Eew::from_value(value).ok_or(ParseError::Incomplete("jma_eew"))?)),
            "jma_eqlist" => Message::EarthquakeList(Box::new(EarthquakeList::from_value(value))),
            "jma_tsunami" => Message::Tsunami(Box::new(Tsunami::from_value(value))),
            "jma_special_eq" => Message::Special(Box::new(SpecialReports::from_value(value))),
            other => Message::Other(other.to_owned()),
        })
    }
}

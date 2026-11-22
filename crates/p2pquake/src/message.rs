use crate::{Earthquake, Eew, Tsunami};
use lenient::Lenient;
use serde_json::Value;
use std::fmt;

/// One message of the API.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// 551: 地震情報.
    Earthquake(Box<Earthquake>),
    /// 552: 津波予報.
    Tsunami(Box<Tsunami>),
    /// 554: an earthquake early warning (warning) was detected being broadcast.
    EewDetection { id: String, time: Option<jma::Time> },
    /// 556: 緊急地震速報（警報）.
    Eew(Box<Eew>),
    /// Another code (555 peers by area, 561 felt reports, 9611 their evaluation, or a new one).
    Other { code: i64, id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Json(String),
    NoCode,
    /// A message of a known code without what identifies it.
    Incomplete(i64),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Json(e) => write!(f, "not JSON: {e}"),
            ParseError::NoCode => f.write_str("no code"),
            ParseError::Incomplete(code) => write!(f, "incomplete message of code {code}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl Message {
    pub const EARTHQUAKE: i64 = 551;
    pub const TSUNAMI: i64 = 552;
    pub const EEW_DETECTION: i64 = 554;
    pub const EEW: i64 = 556;

    /// Reads one message (a frame of the WebSocket API).
    pub fn parse(text: &str) -> Result<Message, ParseError> {
        let value: Value = serde_json::from_str(text).map_err(|e| ParseError::Json(e.to_string()))?;
        Self::from_value(&value)
    }

    /// Reads a response of `/history`: an array of messages. Messages that cannot be read are
    /// left out, with their errors.
    pub fn parse_list(text: &str) -> Result<(Vec<Message>, Vec<ParseError>), ParseError> {
        let value: Value = serde_json::from_str(text).map_err(|e| ParseError::Json(e.to_string()))?;
        let items = value.as_array().ok_or_else(|| ParseError::Json("not an array".into()))?;
        let (mut messages, mut errors) = (Vec::new(), Vec::new());
        for item in items {
            match Self::from_value(item) {
                Ok(message) => messages.push(message),
                Err(error) => errors.push(error),
            }
        }
        Ok((messages, errors))
    }

    pub fn from_value(value: &Value) -> Result<Message, ParseError> {
        let code = value.integer(&["code"]).ok_or(ParseError::NoCode)?;
        let incomplete = ParseError::Incomplete(code);
        Ok(match code {
            Self::EARTHQUAKE => Message::Earthquake(Box::new(Earthquake::from_value(value).ok_or(incomplete)?)),
            Self::TSUNAMI => Message::Tsunami(Box::new(Tsunami::from_value(value).ok_or(incomplete)?)),
            Self::EEW_DETECTION => Message::EewDetection { id: value.text(&["id"]).ok_or(incomplete)?, time: crate::time(value.text(&["time"])) },
            Self::EEW => Message::Eew(Box::new(Eew::from_value(value).ok_or(incomplete)?)),
            code => Message::Other { code, id: value.text(&["id"]) },
        })
    }

    /// When P2P Quake received the information from its source (`time`), for the messages
    /// that carry it.
    pub fn received(&self) -> Option<jma::Time> {
        match self {
            Message::Earthquake(e) => e.received,
            Message::Eew(e) => e.received,
            Message::EewDetection { time, .. } => *time,
            Message::Tsunami(_) | Message::Other { .. } => None,
        }
    }

    /// The `id` of the message, which tells repeated deliveries apart.
    pub fn id(&self) -> Option<&str> {
        match self {
            Message::Earthquake(e) => Some(&e.id),
            Message::Tsunami(t) => Some(&t.id),
            Message::EewDetection { id, .. } => Some(id),
            Message::Eew(e) => Some(&e.id),
            Message::Other { id, .. } => id.as_deref(),
        }
    }
}

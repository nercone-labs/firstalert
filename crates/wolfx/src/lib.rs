//! Messages of the Wolfx Open API (<https://wolfx.jp/docs/open-api>) that carry the information
//! of the Japan Meteorological Agency: earthquake early warnings (`jma_eew`), the list of recent
//! earthquakes (`jma_eqlist`), the state of tsunami warnings (`jma_tsunami`), and the special information
//! about the Nankai Trough and off Hokkaido and Sanriku (`jma_special_eq`).
//!
//! The same JSON comes from the HTTP API (`https://api.wolfx.jp/<source>.json`) and from the
//! WebSocket API (`wss://ws-api.wolfx.jp/<source>`, or `all_eew` for several), which adds a
//! `type` naming the source and sends heartbeats. Fields are read leniently (`lenient`): a field
//! that changes its type is lost alone, never the message.

mod earthquakes;
mod eew;
mod message;
mod special;
mod tsunami;

pub use earthquakes::{EarthquakeEntry, EarthquakeList};
pub use eew::{Eew, WarnArea};
pub use message::{Endpoint, Message, ParseError};
pub use special::{SpecialProduct, SpecialReport, SpecialReports};
pub use tsunami::{Tsunami, TsunamiAreaState, Wave};

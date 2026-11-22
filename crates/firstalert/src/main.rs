//! `firstalert-replay`: feeds messages saved from the relays through the core, and prints what
//! the core would have the platform do. For trying the core on real messages without waiting
//! for an earthquake.
//!
//! ```text
//! firstalert-replay [--place LAT,LON] [--language japanese|easy_japanese|english] [--now MILLIS] FILE...
//! ```
//!
//! Each file holds one message, or a JSON array of messages (a response of P2P Quake's
//! `/history`), or one message per line. A message with a `code` is P2P Quake's; one with a
//! `type` or with Wolfx's fields is Wolfx's. Without `--now`, the time of each message is taken
//! to be just after it was issued, so that it is fresh.

use firstalert::connection::SourceId;
use firstalert::settings::{Language, Place, PlaceSource};
use firstalert::{Core, Effect, Input, Settings};
use geodesy::Coordinate;
use jma::Time;
use lenient::Lenient;
use serde_json::Value;
use std::process::ExitCode;

struct Options {
    place: Option<Coordinate>,
    language: Option<Language>,
    now: Option<Time>,
    files: Vec<String>,
}

impl Options {
    fn parse() -> Result<Options, lexopt::Error> {
        use lexopt::prelude::*;
        let mut options = Options { place: None, language: None, now: None, files: Vec::new() };
        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Long("place") => {
                    let value: String = parser.value()?.parse()?;
                    let (lat, lon) = value.split_once(',').ok_or("--place takes LAT,LON")?;
                    let coordinate = Coordinate::checked(lat.trim().parse().map_err(|_| "bad latitude")?, lon.trim().parse().map_err(|_| "bad longitude")?)
                        .ok_or("--place out of range")?;
                    options.place = Some(coordinate);
                }
                Long("language") => {
                    let value: String = parser.value()?.parse()?;
                    options.language =
                        Some(serde_json::from_value(serde_json::Value::String(value)).map_err(|_| "--language: japanese, easy_japanese or english")?);
                }
                Long("now") => options.now = Some(Time(parser.value()?.parse()?)),
                Value(file) => options.files.push(file.string()?),
                Long("help") | Short('h') => {
                    println!("firstalert-replay [--place LAT,LON] [--language japanese|easy_japanese|english] [--now MILLIS] FILE...");
                    std::process::exit(0);
                }
                _ => return Err(arg.unexpected()),
            }
        }
        Ok(options)
    }
}

/// The messages of a file, each with its relay.
fn messages(text: &str) -> Vec<(SourceId, String)> {
    let values: Vec<Value> = match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(items)) => items,
        Ok(value) => vec![value],
        Err(_) => text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect(),
    };
    values
        .into_iter()
        .map(|value| {
            let source = if value.integer(&["code"]).is_some() { SourceId::P2pquake } else { SourceId::Wolfx };
            let mut value = value;
            // Wolfx's HTTP form has no type: name it by its fields.
            if source == SourceId::Wolfx && value.text(&["type"]).is_none() {
                let kind = if value.at(&["EventID"]).is_some() && value.at(&["Serial"]).is_some() {
                    "jma_eew"
                } else if value.at(&["No1"]).is_some() {
                    "jma_eqlist"
                } else if value.at(&["reports"]).is_some() {
                    "jma_special_eq"
                } else {
                    "jma_tsunami"
                };
                if let Value::Object(map) = &mut value {
                    map.insert("type".into(), Value::String(kind.into()));
                }
            }
            (source, value.to_string())
        })
        .collect()
}

/// The time just after a message was issued.
fn issued(text: &str) -> Option<Time> {
    let value: Value = serde_json::from_str(text).ok()?;
    let jst = ["AnnouncedTime"].iter().chain(&["time"]).find_map(|k| value.text(&[k])).or_else(|| value.text(&["issue", "time"]));
    jst.and_then(|t| Time::parse_jst(&t)).map(|t| Time(t.0 + 1_000))
}

fn main() -> ExitCode {
    let options = match Options::parse() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("firstalert-replay: {error}");
            return ExitCode::from(2);
        }
    };
    let mut core = Core::new();
    let start = options.now.unwrap_or(Time(0));
    core.handle(Input::Start { now: start, data_dir: None, locale: "ja".into() });
    let mut settings = Settings { language: options.language, ..Settings::default() };
    settings.place = options.place.map(|c| Place::at(PlaceSource::Point, c));
    core.handle(Input::Configure { now: start, settings });
    for file in &options.files {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("firstalert-replay: {file}: {error}");
                return ExitCode::FAILURE;
            }
        };
        for (source, message) in messages(&text) {
            let now = options.now.or_else(|| issued(&message)).unwrap_or(start);
            let response = core.handle(Input::Received { now, source, text: message });
            for effect in response.effects {
                match effect {
                    Effect::Notify { notification } => println!("{}", serde_json::to_string_pretty(&notification).unwrap_or_default()),
                    Effect::Log { level, message } => eprintln!("{level:?}: {message}"),
                    _ => {}
                }
            }
        }
    }
    ExitCode::SUCCESS
}

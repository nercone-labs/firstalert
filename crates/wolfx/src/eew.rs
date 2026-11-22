use geodesy::Coordinate;
use jma::{Intensity, IntensityRange, Time};
use lenient::Lenient;
use serde_json::Value;

/// A JMA earthquake early warning (緊急地震速報), forecast or warning, as Wolfx relays it.
#[derive(Debug, Clone, PartialEq)]
pub struct Eew {
    pub title: Option<String>,
    pub event_id: String,
    pub serial: u32,
    /// The issuing office (`Issue.Source`).
    pub source: Option<String>,
    /// `Issue.Status`: `通常` for a real report.
    pub status: Option<String>,
    pub announced: Option<Time>,
    /// The origin time; for a report by the level method, the time of detection.
    pub origin: Option<Time>,
    pub hypocentre: Option<String>,
    pub epicentre: Option<Coordinate>,
    /// The magnitude; `None` when unknown (Wolfx sends 0 for unknown).
    pub magnitude: Option<f64>,
    pub depth_km: Option<f64>,
    pub max_intensity: Option<IntensityRange>,
    pub areas: Vec<WarnArea>,
    pub is_sea: bool,
    pub is_training: bool,
    /// The hypocentre is assumed (仮定震源要素): the magnitude and the depth are fixed values.
    pub is_assumption: bool,
    pub is_warning: bool,
    pub is_final: bool,
    pub is_cancel: bool,
    pub original_text: Option<String>,
}

/// An area of a warning or a forecast (`WarnArea[]`).
#[derive(Debug, Clone, PartialEq)]
pub struct WarnArea {
    pub name: String,
    pub intensity: Option<IntensityRange>,
    /// The forecast arrival of the main shaking.
    pub arrival: Option<Time>,
    /// The area is under a warning (`Type` is `警報`), not only a forecast.
    pub warning: bool,
    /// The main shaking is forecast to have arrived already.
    pub arrived: bool,
    /// `Arrive`, as sent.
    pub arrival_note: Option<String>,
}

/// An intensity range from a maximum and a minimum written as Wolfx writes them (`5弱`, `4`).
/// `以上` after the maximum, or a maximum that is not a class (`～`), leaves the range open.
fn range(max: Option<String>, min: Option<String>) -> Option<IntensityRange> {
    let max = max.map(|m| m.trim().to_owned());
    let open = max.as_deref().is_some_and(|m| m.ends_with("以上") || m.ends_with("程度以上") || m == "～" || m == "~");
    let upper = max.as_deref().map(|m| m.trim_end_matches("程度以上").trim_end_matches("以上")).and_then(Intensity::parse);
    let lower = min.as_deref().and_then(Intensity::parse);
    if open {
        return lower.or(upper).map(IntensityRange::at_least);
    }
    match (lower, upper) {
        (Some(lower), Some(upper)) => Some(IntensityRange { lower: lower.min(upper), upper: Some(upper.max(lower)) }),
        (None, Some(upper)) => Some(IntensityRange::exactly(upper)),
        // A minimum without a maximum: the shaking may be stronger.
        (Some(lower), None) => Some(IntensityRange::at_least(lower)),
        (None, None) => None,
    }
}

/// A time of a warning area: a full JST time, or `HHMMSS` on the day of `reference`, taken on
/// the following day when it would otherwise fall more than an hour before it (a warning just
/// before midnight). `//////` and empty text are unknown.
fn area_time(text: Option<String>, reference: Option<Time>) -> Option<Time> {
    let text = text?;
    let text = text.trim();
    if let Some(time) = Time::parse_jst(text) {
        return Some(time);
    }
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 6 || text.len() != 6 {
        return None;
    }
    let reference = reference?;
    let (year, month, day, ..) = reference.jst_fields();
    let candidate = Time::parse_jst(&format!("{year:04}/{month:02}/{day:02} {}:{}:{}", &digits[0..2], &digits[2..4], &digits[4..6]))?;
    if candidate.0 < reference.0 - 3_600_000 { Some(Time(candidate.0 + 86_400_000)) } else { Some(candidate) }
}

impl Eew {
    /// Reads the JSON of `jma_eew`; `None` without an event ID or a serial number, which every
    /// report has and without which reports cannot be told apart.
    pub fn from_value(value: &Value) -> Option<Eew> {
        let event_id = value.text(&["EventID"])?;
        let serial = value.integer(&["Serial"]).and_then(|s| u32::try_from(s).ok())?;
        let announced = value.text(&["AnnouncedTime"]).and_then(|t| Time::parse_jst(&t));
        let origin = value.text(&["OriginTime"]).and_then(|t| Time::parse_jst(&t));
        let is_assumption = value.boolean(&["isAssumption"]).unwrap_or(false);
        let magnitude = value.number(&["Magnitude"]).or_else(|| value.number(&["Magunitude"])).filter(|m| *m > 0.0);
        let depth_km = value.number(&["Depth"]).filter(|d| *d >= 0.0);
        let epicentre = match (value.number(&["Latitude"]), value.number(&["Longitude"])) {
            (Some(lat), Some(lon)) if !(lat == 0.0 && lon == 0.0) => Coordinate::checked(lat, lon),
            _ => None,
        };
        let reference = announced.or(origin);
        let areas = value
            .elements(&["WarnArea"])
            .iter()
            .filter_map(|area| {
                let name = area.text(&["Chiiki"])?;
                let note = area.text(&["Arrive"]);
                Some(WarnArea {
                    name,
                    intensity: range(area.text(&["Shindo1"]), area.text(&["Shindo2"])),
                    arrival: area_time(area.text(&["Time"]), reference),
                    warning: area.text(&["Type"]).is_some_and(|t| t == "警報"),
                    arrived: note.as_deref().is_some_and(|n| n.contains("既に") || n.contains("到達済") || n == "到達"),
                    arrival_note: note,
                })
            })
            .collect();
        Some(Eew {
            title: value.text(&["Title"]),
            event_id,
            serial,
            source: value.text(&["Issue", "Source"]),
            status: value.text(&["Issue", "Status"]),
            announced,
            origin,
            hypocentre: value.text(&["Hypocenter"]),
            epicentre,
            magnitude,
            depth_km,
            max_intensity: range(value.text(&["MaxIntensity"]), None),
            areas,
            is_sea: value.boolean(&["isSea"]).unwrap_or(false),
            is_training: value.boolean(&["isTraining"]).unwrap_or(false),
            is_assumption,
            is_warning: value.boolean(&["isWarn"]).unwrap_or(false),
            is_final: value.boolean(&["isFinal"]).unwrap_or(false),
            is_cancel: value.boolean(&["isCancel"]).unwrap_or(false),
            original_text: value.text(&["OriginalText"]),
        })
    }
}

use geodesy::Coordinate;
use jma::{Intensity, Time};
use lenient::Lenient;
use serde_json::Value;

/// The list of recent earthquakes (`jma_eqlist`): up to 50, newest first.
#[derive(Debug, Clone, PartialEq)]
pub struct EarthquakeList {
    pub entries: Vec<EarthquakeEntry>,
    /// Changes whenever any entry changes.
    pub md5: Option<String>,
}

/// One earthquake of the list, as its latest report left it.
#[derive(Debug, Clone, PartialEq)]
pub struct EarthquakeEntry {
    /// The position in the list, 1 for the newest.
    pub index: u32,
    pub title: Option<String>,
    pub event_id: Option<String>,
    /// The origin time (for a report of intensities only, the time of detection).
    pub origin: Option<Time>,
    /// The hypocentre; `None` while it is being determined (`震源 調査中`).
    pub location: Option<String>,
    pub magnitude: Option<f64>,
    pub max_intensity: Option<Intensity>,
    /// The depth in km; 0 for very shallow (`ごく浅い`).
    pub depth_km: Option<f64>,
    pub epicentre: Option<Coordinate>,
    /// The forecast comment of JMA, such as whether a tsunami is expected.
    pub info: Option<String>,
}

impl EarthquakeList {
    pub fn from_value(value: &Value) -> EarthquakeList {
        let mut entries: Vec<EarthquakeEntry> = value
            .as_object()
            .map(|object| {
                object
                    .iter()
                    .filter_map(|(key, entry)| {
                        let index: u32 = key.strip_prefix("No")?.parse().ok()?;
                        entry.is_object().then(|| EarthquakeEntry::from_value(index, entry)).flatten()
                    })
                    .collect()
            })
            .unwrap_or_default();
        entries.sort_by_key(|e| e.index);
        EarthquakeList { entries, md5: value.text(&["md5"]) }
    }
}

impl EarthquakeEntry {
    fn from_value(index: u32, value: &Value) -> Option<EarthquakeEntry> {
        let origin = value.text(&["time_full"]).or_else(|| value.text(&["time"])).and_then(|t| Time::parse_jst(&t));
        let event_id = value.text(&["EventID"]);
        // An unused slot has no identity at all.
        if origin.is_none() && event_id.is_none() {
            return None;
        }
        let location = value.text(&["location"]).filter(|l| !l.contains("調査中"));
        let magnitude = value.number(&["magnitude"]).filter(|m| *m >= 0.0);
        let depth_km = value.text(&["depth"]).and_then(|d| {
            if d.contains("ごく浅い") {
                return Some(0.0);
            }
            d.trim_end_matches("km").trim().parse::<f64>().ok().filter(|d| *d >= 0.0)
        });
        let epicentre = match (value.number(&["latitude"]), value.number(&["longitude"])) {
            (Some(lat), Some(lon)) if !(lat == 0.0 && lon == 0.0) && location.is_some() => Coordinate::checked(lat, lon),
            _ => None,
        };
        Some(EarthquakeEntry {
            index,
            title: value.text(&["Title"]),
            event_id,
            origin,
            location,
            magnitude,
            max_intensity: value.text(&["shindo"]).and_then(|s| Intensity::parse(&s)),
            depth_km,
            epicentre,
            info: value.text(&["info"]),
        })
    }
}

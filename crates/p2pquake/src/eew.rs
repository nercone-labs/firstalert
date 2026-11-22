use geodesy::Coordinate;
use jma::{Intensity, IntensityRange, Time};
use lenient::Lenient;
use serde_json::Value;

/// 緊急地震速報（警報） (code 556). P2P Quake relays warnings only, not forecasts.
#[derive(Debug, Clone, PartialEq)]
pub struct Eew {
    pub id: String,
    /// When P2P Quake received it (`time`).
    pub received: Option<Time>,
    pub test: bool,
    pub event_id: String,
    pub serial: u32,
    pub issued: Option<Time>,
    pub cancelled: bool,
    pub origin: Option<Time>,
    /// The time the earthquake was detected (地震発現時刻).
    pub detected: Option<Time>,
    /// The hypocentre is assumed (仮定震源要素).
    pub is_assumption: bool,
    pub hypocentre: Option<String>,
    pub hypocentre_short: Option<String>,
    pub epicentre: Option<Coordinate>,
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
    pub areas: Vec<EewArea>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EewArea {
    /// The forecast region (府県予報区), such as `熊本` or `北海道道北`.
    pub region: String,
    pub name: String,
    pub intensity: Option<IntensityRange>,
    /// `10` not yet arrived, `11` arrived already, `19` no arrival forecast (PLUM method).
    pub kind_code: Option<String>,
    pub arrival: Option<Time>,
}

impl EewArea {
    /// The main shaking is forecast to have arrived already (kind code 11).
    pub fn arrived(&self) -> bool {
        self.kind_code.as_deref() == Some("11")
    }
}

/// The range of `scaleFrom` and `scaleTo`: `99` as the upper bound is `〜程度以上`; `-1` is unknown.
fn range(from: Option<i64>, to: Option<i64>) -> Option<IntensityRange> {
    let lower = from.and_then(Intensity::from_scale);
    match (lower, to) {
        (Some(lower), Some(99)) => Some(IntensityRange::at_least(lower)),
        (Some(lower), to) => match to.and_then(Intensity::from_scale) {
            Some(upper) => Some(IntensityRange { lower: lower.min(upper), upper: Some(upper.max(lower)) }),
            None => Some(IntensityRange::at_least(lower)),
        },
        (None, to) => to.and_then(Intensity::from_scale).map(IntensityRange::exactly),
    }
}

impl Eew {
    pub fn from_value(value: &Value) -> Option<Eew> {
        let id = value.text(&["id"])?;
        let event_id = value.text(&["issue", "eventId"])?;
        let serial = value.integer(&["issue", "serial"]).and_then(|s| u32::try_from(s).ok())?;
        let (epicentre, depth_km, magnitude) = crate::hypocentre(value, &["earthquake", "hypocenter"]);
        let areas = value
            .elements(&["areas"])
            .iter()
            .filter_map(|area| {
                Some(EewArea {
                    region: area.text(&["pref"]).unwrap_or_default(),
                    name: area.text(&["name"])?,
                    intensity: range(area.number(&["scaleFrom"]).map(|s| s.trunc() as i64), area.number(&["scaleTo"]).map(|s| s.trunc() as i64)),
                    kind_code: area.text(&["kindCode"]),
                    arrival: crate::time(area.text(&["arrivalTime"])),
                })
            })
            .collect();
        Some(Eew {
            id,
            received: crate::time(value.text(&["time"])),
            test: value.boolean(&["test"]).unwrap_or(false),
            event_id,
            serial,
            issued: crate::time(value.text(&["issue", "time"])),
            cancelled: value.boolean(&["cancelled"]).unwrap_or(false),
            origin: crate::time(value.text(&["earthquake", "originTime"])),
            detected: crate::time(value.text(&["earthquake", "arrivalTime"])),
            is_assumption: value.text(&["earthquake", "condition"]).is_some_and(|c| c == "仮定震源要素"),
            hypocentre: value.text(&["earthquake", "hypocenter", "name"]),
            hypocentre_short: value.text(&["earthquake", "hypocenter", "reduceName"]),
            epicentre,
            depth_km,
            magnitude,
            areas,
        })
    }
}

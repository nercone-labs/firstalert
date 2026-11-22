use jma::{Time, TsunamiGrade};
use lenient::Lenient;
use serde_json::Value;

/// 津波予報 (code 552). A cancelled report lifts every area (and has none).
#[derive(Debug, Clone, PartialEq)]
pub struct Tsunami {
    pub id: String,
    pub issued: Option<Time>,
    pub source: Option<String>,
    pub cancelled: bool,
    pub areas: Vec<TsunamiArea>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TsunamiArea {
    pub name: String,
    pub grade: TsunamiGrade,
    /// The tsunami is expected to arrive at once.
    pub immediate: bool,
    pub first_arrival: Option<Time>,
    /// The state of the first wave (`ただちに津波来襲と予測`, `第１波の到達を確認`, …).
    pub first_condition: Option<String>,
    /// The expected height as written (`３ｍ`, `巨大`).
    pub max_height_text: Option<String>,
    /// The expected height in metres; `None` for `巨大` and `高い`.
    pub max_height_m: Option<f64>,
}

impl Tsunami {
    pub fn from_value(value: &Value) -> Option<Tsunami> {
        let areas = value
            .elements(&["areas"])
            .iter()
            .filter_map(|area| {
                let grade = match area.text(&["grade"]).as_deref() {
                    Some("MajorWarning") => TsunamiGrade::MajorWarning,
                    Some("Warning") => TsunamiGrade::Warning,
                    Some("Watch") => TsunamiGrade::Advisory,
                    _ => TsunamiGrade::Unknown,
                };
                Some(TsunamiArea {
                    name: area.text(&["name"])?,
                    grade,
                    immediate: area.boolean(&["immediate"]).unwrap_or(false),
                    first_arrival: crate::time(area.text(&["firstHeight", "arrivalTime"])),
                    first_condition: area.text(&["firstHeight", "condition"]),
                    max_height_text: area.text(&["maxHeight", "description"]),
                    max_height_m: area.number(&["maxHeight", "value"]),
                })
            })
            .collect();
        Some(Tsunami {
            id: value.text(&["id"])?,
            issued: crate::time(value.text(&["issue", "time"])),
            source: value.text(&["issue", "source"]),
            cancelled: value.boolean(&["cancelled"]).unwrap_or(false),
            areas,
        })
    }
}

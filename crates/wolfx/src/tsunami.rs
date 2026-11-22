use jma::{Time, TsunamiGrade};
use lenient::Lenient;
use serde_json::Value;

/// The national state of tsunami warnings and forecasts (`jma_tsunami`).
#[derive(Debug, Clone, PartialEq)]
pub struct Tsunami {
    /// `active`, `forecast_only`, `cleared`, `expired`, `none_reported`, `unknown` or
    /// `uninitialized`, as sent. Only `active` and `forecast_only` hold current areas.
    pub status: Option<String>,
    pub trusted: bool,
    /// Until when the state may be trusted without news.
    pub valid_until: Option<Time>,
    pub report_id: Option<String>,
    pub report_time: Option<Time>,
    pub event_ids: Vec<String>,
    pub areas: Vec<TsunamiAreaState>,
    /// The headline comment of JMA (`comments.warning`).
    pub warning_comment: Option<String>,
    pub text: Option<String>,
    pub md5: Option<String>,
}

/// A coastal area under a warning, an advisory or a forecast.
#[derive(Debug, Clone, PartialEq)]
pub struct TsunamiAreaState {
    pub code: Option<String>,
    pub name: String,
    /// `None` for an area whose warning is lifted (`cleared`, `none`).
    pub grade: Option<TsunamiGrade>,
    pub first_wave: Wave,
    pub max_wave: Wave,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wave {
    pub time: Option<Time>,
    /// Metres; `None` for qualitative heights (`巨大`, `高い`).
    pub height_m: Option<f64>,
    /// The height or the state as written (`３ｍ`, `巨大`, `第１波の到達を確認`).
    pub text: Option<String>,
    pub condition: Option<String>,
}

impl Wave {
    fn from_value(value: &Value, key: &str) -> Wave {
        Wave {
            time: value.text(&[key, "time"]).and_then(|t| Time::parse_rfc3339(&t)),
            height_m: value.number(&[key, "height"]),
            text: value.text(&[key, "text"]),
            condition: value.text(&[key, "condition"]).or_else(|| value.text(&[key, "status"])),
        }
    }
}

impl Tsunami {
    pub fn from_value(value: &Value) -> Tsunami {
        let areas = value
            .elements(&["areas"])
            .iter()
            .filter_map(|area| {
                let name = area.text(&["name"])?;
                let grade = match area.text(&["level"]).as_deref() {
                    Some("major_warning") => Some(TsunamiGrade::MajorWarning),
                    Some("warning") => Some(TsunamiGrade::Warning),
                    Some("advisory") => Some(TsunamiGrade::Advisory),
                    Some("forecast") => Some(TsunamiGrade::Forecast),
                    Some("cleared") | Some("none") => None,
                    _ => Some(TsunamiGrade::Unknown),
                };
                Some(TsunamiAreaState {
                    code: area.text(&["code"]),
                    name,
                    grade,
                    first_wave: Wave::from_value(area, "firstWave"),
                    max_wave: Wave::from_value(area, "maxWave"),
                })
            })
            .collect();
        Tsunami {
            status: value.text(&["status"]),
            trusted: value.boolean(&["trusted"]).unwrap_or(false),
            valid_until: value.text(&["validUntil"]).and_then(|t| Time::parse_rfc3339(&t)),
            report_id: value.text(&["report", "id"]),
            report_time: value.text(&["report", "time"]).and_then(|t| Time::parse_rfc3339(&t)),
            event_ids: value.elements(&["report", "eventIds"]).iter().filter_map(|e| e.as_str().map(str::to_owned)).collect(),
            areas,
            warning_comment: value.text(&["comments", "warning"]),
            text: value.text(&["text"]),
            md5: value.text(&["md5"]),
        }
    }

    /// Whether the state holds current areas (`active` or `forecast_only`).
    pub fn is_current(&self) -> bool {
        matches!(self.status.as_deref(), Some("active") | Some("forecast_only"))
    }
}

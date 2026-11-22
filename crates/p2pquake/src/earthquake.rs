use geodesy::Coordinate;
use jma::{Intensity, Time};
use lenient::Lenient;
use serde_json::Value;

/// 地震情報 (code 551).
#[derive(Debug, Clone, PartialEq)]
pub struct Earthquake {
    pub id: String,
    /// When P2P Quake received it.
    pub received: Option<Time>,
    pub issued: Option<Time>,
    pub source: Option<String>,
    pub kind: IssueKind,
    /// What the report corrects (`correct`), when it is a correction.
    pub correction: Option<String>,
    pub origin: Option<Time>,
    pub hypocentre: Option<String>,
    pub epicentre: Option<Coordinate>,
    /// km; 0 for very shallow.
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
    pub max_intensity: Option<Intensity>,
    pub domestic_tsunami: DomesticTsunami,
    pub foreign_tsunami: Option<String>,
    pub points: Vec<ObservationPoint>,
    pub comment: Option<String>,
}

/// The kind of an earthquake report (`issue.type`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    /// 震度速報: the intensities of areas, before the hypocentre is known.
    ScalePrompt,
    /// 震源に関する情報.
    Destination,
    /// 震度・震源に関する情報.
    ScaleAndDestination,
    /// 各地の震度に関する情報.
    DetailScale,
    /// 遠地地震に関する情報.
    Foreign,
    Other(String),
}

impl IssueKind {
    fn parse(text: &str) -> IssueKind {
        match text {
            "ScalePrompt" => IssueKind::ScalePrompt,
            "Destination" => IssueKind::Destination,
            "ScaleAndDestination" => IssueKind::ScaleAndDestination,
            "DetailScale" => IssueKind::DetailScale,
            "Foreign" => IssueKind::Foreign,
            other => IssueKind::Other(other.to_owned()),
        }
    }
}

/// Whether a tsunami is expected in Japan (`domesticTsunami`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomesticTsunami {
    None,
    Unknown,
    Checking,
    /// Slight changes of the sea level, no damage expected.
    NonEffective,
    Watch,
    Warning,
    Other(String),
}

impl DomesticTsunami {
    fn parse(text: Option<String>) -> DomesticTsunami {
        match text.as_deref() {
            Some("None") => DomesticTsunami::None,
            Some("Unknown") | None => DomesticTsunami::Unknown,
            Some("Checking") => DomesticTsunami::Checking,
            Some("NonEffective") => DomesticTsunami::NonEffective,
            Some("Watch") => DomesticTsunami::Watch,
            Some("Warning") => DomesticTsunami::Warning,
            Some(other) => DomesticTsunami::Other(other.to_owned()),
        }
    }
}

/// An intensity observed at a station, or (in 震度速報) in an area.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationPoint {
    pub prefecture: String,
    pub name: String,
    pub is_area: bool,
    pub intensity: Intensity,
    /// The scale was 46: 5弱 or more is estimated, but the intensity has not been received.
    pub estimated: bool,
}

impl Earthquake {
    pub fn from_value(value: &Value) -> Option<Earthquake> {
        let id = value.text(&["id"])?;
        let (epicentre, depth_km, magnitude) = crate::hypocentre(value, &["earthquake", "hypocenter"]);
        let points = value
            .elements(&["points"])
            .iter()
            .filter_map(|point| {
                let scale = point.integer(&["scale"])?;
                Some(ObservationPoint {
                    prefecture: point.text(&["pref"])?,
                    name: point.text(&["addr"])?,
                    is_area: point.boolean(&["isArea"]).unwrap_or(false),
                    intensity: Intensity::from_scale(scale)?,
                    estimated: scale == 46,
                })
            })
            .collect();
        Some(Earthquake {
            id,
            received: crate::time(value.text(&["time"])),
            issued: crate::time(value.text(&["issue", "time"])),
            source: value.text(&["issue", "source"]),
            kind: IssueKind::parse(&value.text(&["issue", "type"]).unwrap_or_default()),
            correction: value.text(&["issue", "correct"]).filter(|c| c != "None"),
            origin: crate::time(value.text(&["earthquake", "time"])),
            hypocentre: value.text(&["earthquake", "hypocenter", "name"]),
            epicentre,
            depth_km,
            magnitude,
            max_intensity: value.integer(&["earthquake", "maxScale"]).and_then(Intensity::from_scale),
            domestic_tsunami: DomesticTsunami::parse(value.text(&["earthquake", "domesticTsunami"])),
            foreign_tsunami: value.text(&["earthquake", "foreignTsunami"]),
            points,
            comment: value.text(&["comments", "freeFormComment"]),
        })
    }
}

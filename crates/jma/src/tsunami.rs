use serde::{Deserialize, Serialize};

/// The kind of a tsunami forecast for a coastal area (津波予報区), from the most to the least
/// severe. The order of the variants is that of severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TsunamiGrade {
    /// A grade the source named but this version does not know. It ranks below every known
    /// grade so as not to be taken for one, but it is shown, never dropped.
    Unknown,
    /// 津波予報（若干の海面変動）: slight changes of the sea level, no damage expected.
    Forecast,
    /// 津波注意報: up to 1 m.
    Advisory,
    /// 津波警報: up to 3 m.
    Warning,
    /// 大津波警報: over 3 m.
    MajorWarning,
}

impl TsunamiGrade {
    /// Whether the grade is a warning or an advisory, which calls for leaving the coast.
    pub fn calls_for_action(&self) -> bool {
        matches!(self, TsunamiGrade::Advisory | TsunamiGrade::Warning | TsunamiGrade::MajorWarning)
    }
}

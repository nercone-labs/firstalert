//! Tsunami warnings, and the special information on large earthquakes.

use crate::connection::SourceId;
use japan::{Prefecture, TsunamiArea};
use jma::{Time, TsunamiGrade};
use serde::{Deserialize, Serialize};

/// A coastal area under a tsunami warning, an advisory or a forecast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsunamiAreaInfo {
    pub name: String,
    pub grade: TsunamiGrade,
    pub prefectures: Vec<Prefecture>,
    pub immediate: bool,
    pub first_arrival: Option<Time>,
    pub first_condition: Option<String>,
    pub max_height: Option<String>,
    pub max_height_m: Option<f64>,
}

/// The tsunami warnings in force, as of the latest report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsunamiState {
    pub issued: Option<Time>,
    pub received: Time,
    /// Every warning, advisory and forecast was lifted.
    pub lifted: bool,
    pub areas: Vec<TsunamiAreaInfo>,
    /// The most severe grade anywhere.
    pub highest: Option<TsunamiGrade>,
    /// The most severe grade on a coast of the user's prefecture.
    pub local: Option<TsunamiGrade>,
    pub comment: Option<String>,
    pub sources: Vec<SourceId>,
}

impl TsunamiState {
    pub fn from_p2pquake(tsunami: &p2pquake::Tsunami, now: Time) -> TsunamiState {
        let areas = if tsunami.cancelled {
            vec![]
        } else {
            tsunami
                .areas
                .iter()
                .map(|a| TsunamiAreaInfo {
                    name: a.name.clone(),
                    grade: a.grade,
                    prefectures: TsunamiArea::prefectures_of(&a.name),
                    immediate: a.immediate,
                    first_arrival: a.first_arrival,
                    first_condition: a.first_condition.clone(),
                    max_height: a.max_height_text.clone(),
                    max_height_m: a.max_height_m,
                })
                .collect()
        };
        Self::new(tsunami.issued, now, tsunami.cancelled, areas, None, SourceId::P2pquake)
    }

    /// The state of Wolfx; `None` when Wolfx says nothing certain of the present (`unknown`,
    /// `uninitialized`, and `expired`, which Wolfx documents as "公式期限切れで解除ではない":
    /// past its official validity, not lifted), which must not be taken for "no warnings".
    pub fn from_wolfx(tsunami: &wolfx::Tsunami, now: Time) -> Option<TsunamiState> {
        let status = tsunami.status.as_deref()?;
        let lifted = match status {
            "active" | "forecast_only" => false,
            "cleared" | "none_reported" => true,
            _ => return None,
        };
        let areas = if lifted {
            vec![]
        } else {
            tsunami
                .areas
                .iter()
                .filter_map(|a| {
                    Some(TsunamiAreaInfo {
                        name: a.name.clone(),
                        grade: a.grade?,
                        prefectures: TsunamiArea::prefectures_of(&a.name),
                        immediate: a.first_wave.condition.as_deref().is_some_and(|c| c.contains("ただちに")),
                        first_arrival: a.first_wave.time,
                        first_condition: a.first_wave.condition.clone(),
                        max_height: a.max_wave.text.clone(),
                        max_height_m: a.max_wave.height_m,
                    })
                })
                .collect()
        };
        Some(Self::new(tsunami.report_time, now, lifted, areas, tsunami.warning_comment.clone(), SourceId::Wolfx))
    }

    fn new(issued: Option<Time>, now: Time, lifted: bool, mut areas: Vec<TsunamiAreaInfo>, comment: Option<String>, source: SourceId) -> TsunamiState {
        areas.sort_by_key(|a| std::cmp::Reverse(a.grade));
        let highest = areas.iter().map(|a| a.grade).max();
        TsunamiState { issued, received: now, lifted, areas, highest, local: None, comment, sources: vec![source] }
    }

    /// Sets the grade of the user's prefecture.
    pub fn localise(&mut self, prefecture: Option<Prefecture>) {
        self.local = prefecture.and_then(|p| self.areas.iter().filter(|a| a.prefectures.contains(&p)).map(|a| a.grade).max());
    }

    /// Whether any warning or advisory is in force.
    pub fn in_force(&self) -> bool {
        !self.lifted && self.highest.is_some_and(|g| g.calls_for_action())
    }

    /// Whether `other` is the same report (the same time of issue and the same areas).
    pub fn same_report(&self, other: &TsunamiState) -> bool {
        self.issued == other.issued
            && self.lifted == other.lifted
            && self.areas.iter().map(|a| (&a.name, a.grade)).eq(other.areas.iter().map(|a| (&a.name, a.grade)))
    }

    /// Whether `other` raises a grade over this: an area more severe than before, or new.
    pub fn raised_by(&self, other: &TsunamiState) -> bool {
        other.areas.iter().any(|a| a.grade.calls_for_action() && self.areas.iter().find(|b| b.name == a.name).is_none_or(|b| a.grade > b.grade))
    }
}

/// A report of the special information (南海トラフ地震臨時情報, 北海道・三陸沖後発地震注意情報,
/// and their commentaries).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecialEntry {
    pub id: String,
    pub product: String,
    pub extraordinary: bool,
    pub time: Option<Time>,
    pub received: Time,
    pub cancelled: bool,
    pub title: Option<String>,
    pub label: Option<String>,
    pub text: Option<String>,
}

impl SpecialEntry {
    /// The longest text kept, in characters: the commentaries can be long.
    pub const TEXT_LIMIT: usize = 4000;

    pub fn from_wolfx(report: &wolfx::SpecialReport, now: Time) -> SpecialEntry {
        let product = match &report.product {
            wolfx::SpecialProduct::NankaiTroughExtra => "VYSE50".to_owned(),
            wolfx::SpecialProduct::NankaiTroughExtraCommentary => "VYSE51".to_owned(),
            wolfx::SpecialProduct::NankaiTroughRelated => "VYSE52".to_owned(),
            wolfx::SpecialProduct::HokkaidoSanrikuSubsequent => "VYSE60".to_owned(),
            wolfx::SpecialProduct::Other(code) => code.clone(),
        };
        SpecialEntry {
            id: report.id.clone(),
            product,
            extraordinary: report.product.is_extraordinary(),
            time: report.time,
            received: now,
            cancelled: report.cancelled,
            title: report.title.clone(),
            label: report.label.clone(),
            text: report.text.as_ref().map(|t| t.chars().take(Self::TEXT_LIMIT).collect()),
        }
    }
}

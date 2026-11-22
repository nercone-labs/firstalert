//! The words of the alerts and the lists, in each language.
//!
//! Every language is a `Phrasebook`. The words aim to be understood at once by anyone: short
//! sentences, the most important first, what to do in plain words, and never only a colour or a
//! number. Intensities are named as JMA names them, and the hypocentre and the time say where
//! and when.

mod easy;
mod english;
mod japanese;

use crate::alert::AlertLevel;
use crate::event::EarthquakeEvent;
use crate::settings::{Language, Place};
use crate::tsunami::{SpecialEntry, TsunamiState};
use japan::{Prefecture, SeismicArea};
use jma::{Intensity, IntensityRange, Time};
use serde::{Deserialize, Serialize};

/// The words of an earthquake in a list.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EventText {
    /// Where: the hypocentre, or that it is being determined.
    pub title: String,
    /// When, how large, how deep.
    pub subtitle: String,
    /// The strongest intensity: observed, or forecast.
    pub intensity: Option<String>,
    /// What the latest information is.
    pub status: String,
    /// What it means at the user's place.
    pub local: Option<String>,
}

/// The words of one alert.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Composed {
    pub title: String,
    pub body: String,
    pub actions: Option<String>,
    pub speech: Option<String>,
}

/// An intensity class with its name and what it feels like, for legends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntensityLabel {
    pub intensity: Intensity,
    pub name: String,
    pub short: String,
    pub description: String,
}

pub trait Phrasebook: Sync {
    fn language(&self) -> Language;

    /// `震度5弱`.
    fn intensity(&self, intensity: Intensity) -> String;
    /// `5弱`: the class alone, for a badge.
    fn intensity_short(&self, intensity: Intensity) -> String;
    /// `震度4から5強`, `震度5弱以上`.
    fn intensity_range(&self, range: IntensityRange) -> String;
    /// What people feel and see at the class.
    fn intensity_feel(&self, intensity: Intensity) -> &'static str;
    /// A date and time of the day, in Japan Standard Time.
    fn time(&self, time: Time) -> String;
    fn magnitude(&self, magnitude: f64) -> String;
    fn depth(&self, depth_km: f64) -> String;
    /// The name of a prefecture.
    fn prefecture(&self, prefecture: Prefecture) -> String;

    fn event_text(&self, event: &EarthquakeEvent) -> EventText;
    fn eew(&self, event: &EarthquakeEvent, level: AlertLevel, place: Option<&str>) -> Composed;
    fn report(&self, event: &EarthquakeEvent, place: Option<&str>) -> Composed;
    fn tsunami(&self, state: &TsunamiState, place: Option<Prefecture>) -> Composed;
    fn special(&self, entry: &SpecialEntry) -> Composed;
    fn test(&self, level: AlertLevel) -> Composed;

    /// The name of the user's place: the area, or the prefecture, or the coordinate.
    fn place(&self, place: &Place) -> String {
        if let Some(area) = place.area.as_deref().and_then(SeismicArea::find) {
            return area.name.to_owned();
        }
        match place.prefecture {
            Some(p) => self.prefecture(p),
            None => format!("{:.2}, {:.2}", place.coordinate.latitude, place.coordinate.longitude),
        }
    }

    fn intensity_labels(&self) -> Vec<IntensityLabel> {
        Intensity::ALL
            .iter()
            .map(|&i| IntensityLabel { intensity: i, name: self.intensity(i), short: self.intensity_short(i), description: self.intensity_feel(i).to_owned() })
            .collect()
    }
}

pub fn phrasebook(language: Language) -> &'static dyn Phrasebook {
    match language {
        Language::Japanese => &japanese::Japanese,
        Language::EasyJapanese => &easy::EasyJapanese,
        Language::English => &english::English,
    }
}

/// The parts of a date and time in JST: month, day, hour, minute.
fn parts(time: Time) -> (u32, u32, u32, u32) {
    let (_, month, day, hour, minute, _) = time.jst_fields();
    (month, day, hour, minute)
}

/// A magnitude to one decimal place.
fn one_decimal(value: f64) -> String {
    format!("{value:.1}")
}

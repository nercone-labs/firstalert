//! The settings of the user, which the core keeps (`settings.json`) so that every platform
//! behaves the same.

use geodesy::Coordinate;
use japan::{Boundaries, Prefecture, SeismicArea};
use jma::{Intensity, SiteAmplification};
use serde::{Deserialize, Serialize};

/// The language of the words of the alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Japanese,
    /// やさしい日本語: short sentences, common words, readings of the harder words.
    EasyJapanese,
    English,
}

impl Language {
    /// The language of a BCP 47 locale: Japanese for `ja`, English otherwise.
    pub fn of_locale(locale: &str) -> Language {
        if locale.to_ascii_lowercase().starts_with("ja") { Language::Japanese } else { Language::English }
    }
}

/// How the user's place was set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceSource {
    /// Chosen as a prefecture: its representative point.
    Prefecture,
    /// A point chosen on the map or entered.
    Point,
    /// The location of the device, updated by the platform.
    Device,
}

/// Where the user is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub source: PlaceSource,
    pub coordinate: Coordinate,
    /// The prefecture; found from the coordinate when not given.
    pub prefecture: Option<Prefecture>,
    /// The area of the warnings (細分区域) when the user chose one. Without it, every area of
    /// the prefecture is the user's, which may alert more often but never less.
    pub area: Option<String>,
}

impl Place {
    /// A place at `coordinate`, with the prefecture that holds it (or whose coast is within
    /// 10 km of it).
    pub fn at(source: PlaceSource, coordinate: Coordinate) -> Place {
        Place { source, coordinate, prefecture: Boundaries::built_in().locate_near(&coordinate, 10.0), area: None }
    }

    /// The representative point of `prefecture`.
    pub fn of_prefecture(prefecture: Prefecture) -> Option<Place> {
        let shape = Boundaries::built_in().shape(prefecture)?;
        Some(Place { source: PlaceSource::Prefecture, coordinate: shape.representative_point(), prefecture: Some(prefecture), area: None })
    }

    /// The place with its area checked: an area that is not one of the prefecture's is dropped.
    pub fn validated(mut self) -> Place {
        if self.prefecture.is_none() {
            self.prefecture = Boundaries::built_in().locate_near(&self.coordinate, 10.0);
        }
        if let Some(area) = &self.area {
            let valid = SeismicArea::find(area).is_some_and(|a| Some(a.prefecture) == self.prefecture);
            if !valid {
                self.area = None;
            }
        }
        self
    }
}

/// The ground under the user's place, as an amplification of the shaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ground {
    /// Rock or firm ground (the reference, Vs ≈ 400 m/s).
    Firm,
    /// Ordinary ground of plains and terraces.
    Average,
    /// Soft ground: reclaimed land, river deltas, former marshes.
    Soft,
}

impl Ground {
    /// The factor of the peak ground velocity relative to firm ground. Softer ground shakes
    /// more; these round factors are a rough choice of FirstAlert for the user to pick from, not
    /// a survey of the site, and the estimate stays a guide either way.
    pub fn amplification(&self) -> SiteAmplification {
        match self {
            Ground::Firm => SiteAmplification(1.0),
            Ground::Average => SiteAmplification(1.5),
            Ground::Soft => SiteAmplification(2.0),
        }
    }
}

/// Which relays to receive from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sources {
    pub wolfx: bool,
    pub p2pquake: bool,
}

impl Default for Sources {
    fn default() -> Self {
        Self { wolfx: true, p2pquake: true }
    }
}

/// From which intensity each kind of information alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    /// An earthquake early warning that expects at least this at the user's place is an
    /// emergency: the strongest alert.
    pub eew_emergency: Intensity,
    /// An early warning that expects at least this at the user's place alerts with sound.
    pub eew_local: Intensity,
    /// An early warning that expects at least this anywhere is shown, quietly.
    pub eew_anywhere: Intensity,
    /// An earthquake report of at least this maximum intensity is shown, quietly.
    pub report_anywhere: Intensity,
    /// An earthquake report of at least this in the user's prefecture alerts with sound.
    pub report_local: Intensity,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            eew_emergency: Intensity::Four,
            eew_local: Intensity::Three,
            eew_anywhere: Intensity::FiveLower,
            report_anywhere: Intensity::Three,
            report_local: Intensity::Three,
        }
    }
}

/// How alerts reach the user. Each is a request to the platform, which does what it can.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Presentation {
    pub sound: bool,
    /// Emergencies sound even when the device is silenced (as an alarm).
    pub sound_when_silenced: bool,
    pub voice: bool,
    pub vibration: bool,
    /// Flash the camera light during emergencies, for users who may not hear the sound.
    pub flash: bool,
    /// Show emergencies over the whole screen, also on the lock screen.
    pub full_screen: bool,
}

impl Default for Presentation {
    fn default() -> Self {
        Self { sound: true, sound_when_silenced: true, voice: true, vibration: true, flash: false, full_screen: true }
    }
}

/// Every setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// `None` follows the language of the system.
    pub language: Option<Language>,
    pub place: Option<Place>,
    pub ground: Ground,
    pub sources: Sources,
    pub thresholds: Thresholds,
    pub tsunami: bool,
    /// The extraordinary information on the Nankai Trough and off Hokkaido and Sanriku.
    pub special: bool,
    /// Show training and test reports. They never alert as real ones do.
    pub training: bool,
    pub presentation: Presentation,
}

impl Settings {
    pub const VERSION: u32 = 1;
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            language: None,
            place: None,
            ground: Ground::Average,
            sources: Sources::default(),
            thresholds: Thresholds::default(),
            tsunami: true,
            special: true,
            training: false,
            presentation: Presentation::default(),
        }
    }
}

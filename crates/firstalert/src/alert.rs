//! How strongly to alert, and the alert itself.

use crate::event::EarthquakeEvent;
use crate::settings::Settings;
use crate::tsunami::{SpecialEntry, TsunamiState};
use jma::{Intensity, Time, TsunamiGrade};
use serde::{Deserialize, Serialize};

/// How strongly something alerts, from not at all to the strongest. The order of the variants
/// is the order of strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    /// Kept in the history only.
    #[default]
    None,
    /// A quiet notification.
    Information,
    /// A notification with sound and vibration.
    Warning,
    /// Everything at once: alarm sound, voice, vibration, the whole screen, the light.
    Emergency,
}

/// What a notification is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Eew,
    Earthquake,
    Tsunami,
    Special,
    Test,
}

/// A sound or a vibration pattern of the platform, by level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Emergency,
    Warning,
    Information,
}

/// A notification for the platform to show, with how to present it. The platform shows what it
/// can and ignores what it cannot (a desktop has no vibration).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    /// Notifications of the same key replace each other.
    pub key: String,
    pub category: Category,
    pub level: AlertLevel,
    /// Sound, voice and vibration this time. An update that adds nothing is shown quietly.
    pub alerting: bool,
    pub title: String,
    pub body: String,
    pub actions: Option<String>,
    /// The text to read aloud.
    pub speech: Option<String>,
    /// The intensity to show as a badge, and whether it is the user's place's.
    pub intensity: Option<Intensity>,
    pub intensity_label: Option<String>,
    pub local: bool,
    pub tsunami: Option<TsunamiGrade>,
    /// When the strong shaking reaches the user, for a countdown.
    pub arrival: Option<Time>,
    pub time: Time,
    pub sound: Option<Pattern>,
    pub sound_when_silenced: bool,
    pub voice: bool,
    pub vibration: Option<Pattern>,
    pub flash: bool,
    pub full_screen: bool,
    /// Kept until the user dismisses it.
    pub ongoing: bool,
    /// The event or state it is about (`Input::Detail` for an earthquake).
    pub subject: Option<String>,
}

impl Notification {
    /// The presentation of `level` with `settings`, when `alerting`.
    pub fn present(&mut self, settings: &Settings) {
        let p = &settings.presentation;
        let pattern = match self.level {
            AlertLevel::Emergency => Some(Pattern::Emergency),
            AlertLevel::Warning => Some(Pattern::Warning),
            AlertLevel::Information => Some(Pattern::Information),
            AlertLevel::None => None,
        };
        let loud = self.alerting && self.level >= AlertLevel::Warning;
        self.sound = if loud && p.sound { pattern } else { None };
        self.sound_when_silenced = loud && p.sound && p.sound_when_silenced && self.level == AlertLevel::Emergency;
        self.voice = loud && p.voice && self.speech.is_some();
        self.vibration = if loud && p.vibration { pattern } else { None };
        self.flash = loud && p.flash && self.level == AlertLevel::Emergency;
        self.full_screen = self.alerting && p.full_screen && self.level == AlertLevel::Emergency;
        self.ongoing = self.level == AlertLevel::Emergency;
    }
}

/// The levels of the kinds of information, from the settings.
pub struct Policy<'a> {
    pub settings: &'a Settings,
    pub now: Time,
}

impl Policy<'_> {
    /// An early warning is fresh this long after it was announced; an older one (received after
    /// the app started, or late) is recorded without alerting.
    pub const EEW_FRESH_MS: i64 = 180_000;
    /// An earthquake report is fresh this long after it was issued.
    pub const REPORT_FRESH_MS: i64 = 30 * 60_000;
    /// A tsunami report is fresh this long after it was issued.
    pub const TSUNAMI_FRESH_MS: i64 = 6 * 3_600_000;
    /// Special information is fresh this long after it was issued.
    pub const SPECIAL_FRESH_MS: i64 = 12 * 3_600_000;

    fn fresh(&self, time: Option<Time>, window: i64) -> bool {
        // A time in the future (a clock behind the servers') is fresh.
        time.is_some_and(|t| self.now.0 - t.0 <= window)
    }

    pub fn eew(&self, event: &EarthquakeEvent) -> AlertLevel {
        let Some(eew) = &event.eew else { return AlertLevel::None };
        if eew.training && !self.settings.training {
            return AlertLevel::None;
        }
        if !self.fresh(eew.announced.or(Some(eew.received)), Self::EEW_FRESH_MS) {
            return AlertLevel::None;
        }
        if eew.cancelled {
            // A cancellation is said quietly, and only for an alert that was given.
            return if event.level > AlertLevel::None { AlertLevel::Information } else { AlertLevel::None };
        }
        let level = self.eew_level(event);
        if eew.training { level.min(AlertLevel::Information) } else { level }
    }

    fn eew_level(&self, event: &EarthquakeEvent) -> AlertLevel {
        let th = &self.settings.thresholds;
        let eew = event.eew.as_ref().expect("checked by the caller");
        let expected = event.local.as_ref().and_then(|l| l.expected);
        if expected.is_some_and(|e| e >= th.eew_emergency) {
            return AlertLevel::Emergency;
        }
        if expected.is_some_and(|e| e >= th.eew_local) {
            return AlertLevel::Warning;
        }
        // A warning stays a warning when a later serial of the same earthquake is a forecast.
        let warned = eew.warning || eew.ever_warning;
        // Without a place, a warning may be for the user: alert with sound.
        if self.settings.place.is_none() && warned {
            return AlertLevel::Warning;
        }
        let anywhere = eew.max_intensity.map(|r| r.strongest_known());
        if warned || anywhere.is_some_and(|m| m >= th.eew_anywhere) {
            return AlertLevel::Information;
        }
        AlertLevel::None
    }

    pub fn report(&self, event: &EarthquakeEvent) -> AlertLevel {
        let Some(report) = &event.report else { return AlertLevel::None };
        let issued = report.issued.or(Some(report.received));
        let origin_fresh = self.fresh(event.origin, Self::REPORT_FRESH_MS);
        if !(self.fresh(issued, Self::REPORT_FRESH_MS) && (event.origin.is_none() || origin_fresh)) {
            return AlertLevel::None;
        }
        let th = &self.settings.thresholds;
        let local = event.local.as_ref().and_then(|l| l.observed);
        if local.is_some_and(|i| i >= th.report_local) {
            return AlertLevel::Warning;
        }
        if report.max_intensity.is_some_and(|m| m >= th.report_anywhere) {
            return AlertLevel::Information;
        }
        AlertLevel::None
    }

    pub fn tsunami(&self, state: &TsunamiState) -> AlertLevel {
        if !self.settings.tsunami || !self.fresh(state.issued.or(Some(state.received)), Self::TSUNAMI_FRESH_MS) {
            return AlertLevel::None;
        }
        if state.lifted {
            return AlertLevel::Information;
        }
        match (state.local, state.highest) {
            (Some(TsunamiGrade::MajorWarning | TsunamiGrade::Warning), _) => AlertLevel::Emergency,
            (Some(TsunamiGrade::Advisory), _) => AlertLevel::Warning,
            (_, Some(TsunamiGrade::MajorWarning | TsunamiGrade::Warning)) => AlertLevel::Warning,
            (_, Some(TsunamiGrade::Advisory)) => AlertLevel::Information,
            _ => AlertLevel::None,
        }
    }

    pub fn special(&self, entry: &SpecialEntry) -> AlertLevel {
        if !self.settings.special || !entry.extraordinary || entry.cancelled || !self.fresh(entry.time.or(Some(entry.received)), Self::SPECIAL_FRESH_MS) {
            return AlertLevel::None;
        }
        AlertLevel::Warning
    }
}

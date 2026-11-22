use super::{Composed, EventText, Phrasebook, one_decimal, parts};
use crate::alert::AlertLevel;
use crate::event::{EarthquakeEvent, ReportKind, TsunamiOutlook};
use crate::settings::Language;
use crate::tsunami::{SpecialEntry, TsunamiState};
use japan::Prefecture;
use jma::{Intensity, IntensityRange, Time, TsunamiGrade};

pub struct English;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

impl English {
    fn grade(grade: TsunamiGrade) -> &'static str {
        match grade {
            TsunamiGrade::MajorWarning => "Major Tsunami Warning",
            TsunamiGrade::Warning => "Tsunami Warning",
            TsunamiGrade::Advisory => "Tsunami Advisory",
            TsunamiGrade::Forecast => "Tsunami Forecast (slight sea-level change)",
            TsunamiGrade::Unknown => "Tsunami information",
        }
    }

    fn place_of(event: &EarthquakeEvent) -> String {
        // Hypocentre names come from JMA in Japanese only.
        event.hypocentre.name.clone().unwrap_or_else(|| "Epicenter under investigation".to_owned())
    }
}

impl Phrasebook for English {
    fn language(&self) -> Language {
        Language::English
    }

    fn intensity(&self, intensity: Intensity) -> String {
        format!("Shindo {}", self.intensity_short(intensity))
    }

    fn intensity_short(&self, intensity: Intensity) -> String {
        match intensity {
            Intensity::Zero => "0",
            Intensity::One => "1",
            Intensity::Two => "2",
            Intensity::Three => "3",
            Intensity::Four => "4",
            Intensity::FiveLower => "5-",
            Intensity::FiveUpper => "5+",
            Intensity::SixLower => "6-",
            Intensity::SixUpper => "6+",
            Intensity::Seven => "7",
        }
        .to_owned()
    }

    fn intensity_range(&self, range: IntensityRange) -> String {
        match range.upper {
            None => format!("{} or more", self.intensity(range.lower)),
            Some(upper) if upper == range.lower => self.intensity(upper),
            Some(upper) => format!("{} to {}", self.intensity(range.lower), self.intensity_short(upper)),
        }
    }

    fn intensity_feel(&self, intensity: Intensity) -> &'static str {
        match intensity {
            Intensity::Zero => "Not felt.",
            Intensity::One => "Felt slightly by some people keeping quiet indoors.",
            Intensity::Two => "Felt by many people keeping quiet indoors. Hanging lamps sway slightly.",
            Intensity::Three => "Felt by most people indoors. Dishes in cupboards may rattle.",
            Intensity::Four => "Most people are startled. Hanging objects swing; unstable ornaments may fall.",
            Intensity::FiveLower => "Many people are frightened and want to hold on to something. Dishes and books may fall.",
            Intensity::FiveUpper => "Hard to walk without holding on. Unsecured furniture may topple.",
            Intensity::SixLower => "Hard to keep standing. Much unsecured furniture moves, and some topples.",
            Intensity::SixUpper => "Impossible to move without crawling. Most unsecured furniture moves and topples.",
            Intensity::Seven => "Thrown about by the shaking. Many buildings of low earthquake resistance collapse.",
        }
    }

    fn time(&self, time: Time) -> String {
        let (month, day, hour, minute) = parts(time);
        format!("{} {day}, {hour:02}:{minute:02} JST", MONTHS[(month as usize).clamp(1, 12) - 1])
    }

    fn magnitude(&self, magnitude: f64) -> String {
        format!("M{}", one_decimal(magnitude))
    }

    fn depth(&self, depth_km: f64) -> String {
        if depth_km <= 0.0 { "very shallow".to_owned() } else { format!("{} km deep", depth_km.round() as i64) }
    }

    fn prefecture(&self, prefecture: Prefecture) -> String {
        prefecture.english_name().to_owned()
    }

    fn event_text(&self, event: &EarthquakeEvent) -> EventText {
        let mut subtitle = Vec::new();
        if let Some(origin) = event.origin {
            subtitle.push(self.time(origin));
        }
        if let Some(m) = event.hypocentre.magnitude {
            subtitle.push(self.magnitude(m));
        }
        if let Some(d) = event.hypocentre.depth_km {
            subtitle.push(self.depth(d));
        }
        let status = match (&event.report, &event.eew) {
            (Some(report), _) => match report.kind {
                ReportKind::Intensities => "Seismic intensity report",
                ReportKind::Hypocentre => "Hypocenter report",
                ReportKind::HypocentreAndIntensities => "Hypocenter and intensity report",
                ReportKind::Detail => "Detailed intensity report",
                ReportKind::Foreign => "Distant earthquake report",
                ReportKind::Other => "Earthquake report",
            }
            .to_owned(),
            (None, Some(eew)) => {
                let kind = if eew.cancelled {
                    "Early Warning (cancelled)"
                } else if eew.ever_warning {
                    "Earthquake Early Warning"
                } else {
                    "Early Warning (forecast)"
                };
                let training = if eew.training { "[Drill] " } else { "" };
                format!("{training}{kind} #{}{}", eew.serial, if eew.final_report { ", final" } else { "" })
            }
            (None, None) => "Earthquake".to_owned(),
        };
        let intensity = match (&event.report, &event.eew) {
            (Some(r), _) => r.max_intensity.map(|i| format!("Max {}", self.intensity(i))),
            (None, Some(e)) => e.max_intensity.map(|r| format!("Expected max {}", self.intensity_range(r))),
            _ => None,
        };
        let local = event.local.as_ref().and_then(|l| match (l.observed, l.expected) {
            (Some(observed), _) => Some(format!("Your area: {}", self.intensity(observed))),
            (None, Some(expected)) if event.report.is_none() => Some(format!("Your area: about {} expected", self.intensity(expected))),
            _ => None,
        });
        EventText { title: Self::place_of(event), subtitle: subtitle.join(" · "), intensity, status, local }
    }

    fn eew(&self, event: &EarthquakeEvent, level: AlertLevel, place: Option<&str>) -> Composed {
        let eew = event.eew.as_ref();
        let prefix = if eew.is_some_and(|e| e.training) { "[Drill] " } else { "" };
        if eew.is_some_and(|e| e.cancelled) {
            return Composed {
                title: format!("{prefix}Early Warning cancelled"),
                body: "The earlier Earthquake Early Warning was cancelled. No strong shaking is expected from it.".to_owned(),
                actions: None,
                speech: Some("The earlier earthquake early warning was cancelled.".to_owned()),
            };
        }
        // A forecast of no shaking says nothing of the user's place.
        let expected = event.local.as_ref().and_then(|l| l.expected).filter(|e| *e >= Intensity::One);
        let title = match level {
            AlertLevel::Emergency => format!("{prefix}Earthquake Early Warning: strong shaking"),
            _ => format!("{prefix}Earthquake Early Warning"),
        };
        let mut body = vec![format!("Earthquake near {}.", Self::place_of(event))];
        match (place, expected) {
            (Some(place), Some(expected)) => body.push(format!("About {} expected in {place}.", self.intensity(expected))),
            _ => {
                if let Some(max) = eew.and_then(|e| e.max_intensity) {
                    body.push(format!("Up to {} expected.", self.intensity_range(max)));
                }
            }
        }
        if let Some(m) = event.hypocentre.magnitude {
            body.push(format!("Estimated {}.", self.magnitude(m)));
        }
        let actions = (level >= AlertLevel::Warning).then(|| {
            "Protect your head. Get under a sturdy table or to a place where nothing can fall, topple or slide onto you, and stay until the shaking stops. If driving, turn on the hazard lights and slow down gently.".to_owned()
        });
        let speech = (level >= AlertLevel::Warning).then(|| match (place, expected) {
            (Some(place), Some(expected)) if level == AlertLevel::Emergency => {
                format!("Earthquake early warning. Strong shaking expected. {} in {place}. Protect your head.", self.intensity(expected))
            }
            _ => "Earthquake early warning. Be ready for shaking. Protect your head.".to_owned(),
        });
        Composed { title, body: body.join(" "), actions, speech }
    }

    fn report(&self, event: &EarthquakeEvent, place: Option<&str>) -> Composed {
        let Some(report) = &event.report else { return Composed::default() };
        let title = match report.max_intensity {
            Some(max) => format!("Earthquake: max {} · {}", self.intensity(max), Self::place_of(event)),
            None => format!("Earthquake · {}", Self::place_of(event)),
        };
        let mut body = Vec::new();
        let when = event.origin.map(|t| format!(" at {}", self.time(t))).unwrap_or_default();
        body.push(format!("An earthquake occurred{when} near {}.", Self::place_of(event)));
        if let Some(max) = report.max_intensity {
            body.push(format!("The strongest shaking was {}.", self.intensity(max)));
        }
        let mut detail = Vec::new();
        if let Some(m) = event.hypocentre.magnitude {
            detail.push(self.magnitude(m));
        }
        if let Some(d) = event.hypocentre.depth_km {
            detail.push(self.depth(d));
        }
        if !detail.is_empty() {
            body.push(format!("{}.", detail.join(", ")));
        }
        if let (Some(place), Some(observed)) = (place, event.local.as_ref().and_then(|l| l.observed)) {
            body.push(format!("{} in {place}.", self.intensity(observed)));
        }
        match report.tsunami {
            TsunamiOutlook::None => body.push("No tsunami is expected from this earthquake.".to_owned()),
            TsunamiOutlook::Slight => body.push("A slight change of the sea level is possible, with no damage expected.".to_owned()),
            TsunamiOutlook::Checking => body.push("Whether a tsunami will follow is being checked. Near the sea, watch for updates.".to_owned()),
            TsunamiOutlook::Advisory => body.push("A tsunami advisory is in effect. Leave the water and the shore.".to_owned()),
            TsunamiOutlook::Warning => body.push("A tsunami warning is in effect. Move to high ground now.".to_owned()),
            TsunamiOutlook::Unknown => {}
        }
        let speech = Some(body.join(" "));
        Composed { title, body: body.join(" "), actions: None, speech }
    }

    fn tsunami(&self, state: &TsunamiState, place: Option<Prefecture>) -> Composed {
        if state.lifted || state.highest.is_none() {
            return Composed {
                title: "Tsunami warnings lifted".to_owned(),
                body: "All tsunami warnings and advisories have been lifted. Near the sea, small changes of the sea level may continue for a while.".to_owned(),
                actions: None,
                speech: Some("All tsunami warnings and advisories have been lifted.".to_owned()),
            };
        }
        let grade = state.local.filter(|g| g.calls_for_action()).or(state.highest).unwrap_or(TsunamiGrade::Unknown);
        let mut body = Vec::new();
        if let (Some(p), Some(local)) = (place, state.local) {
            body.push(format!("{} for the coast of {}.", Self::grade(local), self.prefecture(p)));
        }
        for g in [TsunamiGrade::MajorWarning, TsunamiGrade::Warning, TsunamiGrade::Advisory] {
            let areas: Vec<&str> = state.areas.iter().filter(|a| a.grade == g).map(|a| a.name.as_str()).collect();
            if !areas.is_empty() {
                body.push(format!("{}: {}.", Self::grade(g), areas.join(", ")));
            }
        }
        let actions = match grade {
            TsunamiGrade::MajorWarning | TsunamiGrade::Warning => Some(
                "If you are near the sea or a river, move to high ground at once. Tsunamis come again and again: do not return until the warning is lifted."
                    .to_owned(),
            ),
            TsunamiGrade::Advisory => Some("Get out of the water and away from the shore now.".to_owned()),
            _ => None,
        };
        Composed {
            title: Self::grade(grade).to_owned(),
            body: body.join(" "),
            actions: actions.clone(),
            speech: Some(format!("{}. {}", Self::grade(grade), actions.unwrap_or_default())),
        }
    }

    fn special(&self, entry: &SpecialEntry) -> Composed {
        let title = match entry.product.as_str() {
            "VYSE50" => "Nankai Trough Earthquake Extra Information",
            "VYSE60" => "Subsequent Earthquake Advisory (off Hokkaido and Sanriku)",
            _ => "Special earthquake information",
        };
        Composed {
            title: title.to_owned(),
            body: format!("{} {}", entry.title.clone().unwrap_or_default(), entry.label.clone().unwrap_or_default()).trim().to_owned(),
            actions: Some("Check the announcements of JMA and your municipality. Check your evacuation site and route, secure furniture, and keep water and food at hand.".to_owned()),
            speech: Some(format!("{title} has been issued.")),
        }
    }

    fn test(&self, level: AlertLevel) -> Composed {
        let title = match level {
            AlertLevel::Emergency => "Test: Earthquake Early Warning",
            AlertLevel::Warning => "Test: be ready for shaking",
            _ => "Test: information",
        };
        Composed {
            title: title.to_owned(),
            body: "This is a test. There is no earthquake.".to_owned(),
            actions: None,
            speech: Some("This is a test.".to_owned()),
        }
    }
}

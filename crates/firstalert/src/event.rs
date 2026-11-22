//! Earthquakes: what the early warnings and the reports of both relays say of one earthquake,
//! joined into one event.

use crate::alert::AlertLevel;
use crate::connection::SourceId;
use crate::impact::LocalImpact;
use crate::words::EventText;
use geodesy::Coordinate;
use japan::{Prefecture, SeismicArea};
use jma::{Intensity, IntensityRange, Time};
use serde::{Deserialize, Serialize};

/// Where an earthquake was, as far as known.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Hypocentre {
    pub name: Option<String>,
    pub epicentre: Option<Coordinate>,
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
}

impl Hypocentre {
    /// Fills what this lacks from `other`.
    fn fill(&mut self, other: &Hypocentre) {
        if self.name.is_none() {
            self.name.clone_from(&other.name);
        }
        if self.epicentre.is_none() {
            self.epicentre = other.epicentre;
        }
        if self.depth_km.is_none() {
            self.depth_km = other.depth_km;
        }
        if self.magnitude.is_none() {
            self.magnitude = other.magnitude;
        }
    }
}

/// An area of an early warning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EewArea {
    pub name: String,
    pub prefecture: Option<Prefecture>,
    pub intensity: Option<IntensityRange>,
    pub arrival: Option<Time>,
    /// Under a warning, not only a forecast.
    pub warning: bool,
    pub arrived: bool,
}

/// One report of an early warning, from either relay.
#[derive(Debug, Clone, PartialEq)]
pub struct EewReport {
    pub source: SourceId,
    pub event_id: String,
    pub serial: u32,
    pub announced: Option<Time>,
    pub origin: Option<Time>,
    pub hypocentre: Hypocentre,
    pub warning: bool,
    pub final_report: bool,
    pub cancelled: bool,
    pub training: bool,
    pub assumption: bool,
    pub max_intensity: Option<IntensityRange>,
    pub areas: Vec<EewArea>,
}

impl EewReport {
    pub fn from_wolfx(eew: &wolfx::Eew) -> EewReport {
        let training = eew.is_training || eew.status.as_deref().is_some_and(|s| s != "通常");
        EewReport {
            source: SourceId::Wolfx,
            event_id: eew.event_id.clone(),
            serial: eew.serial,
            announced: eew.announced,
            origin: eew.origin,
            hypocentre: Hypocentre {
                name: eew.hypocentre.clone(),
                epicentre: eew.epicentre,
                // An assumed hypocentre has a fixed depth and magnitude, which are not values.
                depth_km: eew.depth_km.filter(|_| !eew.is_assumption),
                magnitude: eew.magnitude.filter(|_| !eew.is_assumption),
            },
            warning: eew.is_warning,
            final_report: eew.is_final,
            cancelled: eew.is_cancel,
            training,
            assumption: eew.is_assumption,
            max_intensity: eew.max_intensity,
            areas: eew
                .areas
                .iter()
                .map(|a| EewArea {
                    name: a.name.clone(),
                    prefecture: SeismicArea::prefecture_of(&a.name),
                    intensity: a.intensity,
                    arrival: a.arrival,
                    warning: a.warning,
                    arrived: a.arrived,
                })
                .collect(),
        }
    }

    pub fn from_p2pquake(eew: &p2pquake::Eew) -> EewReport {
        let areas: Vec<EewArea> = eew
            .areas
            .iter()
            .map(|a| EewArea {
                name: a.name.clone(),
                prefecture: SeismicArea::prefecture_of_region(&a.region).or_else(|| SeismicArea::prefecture_of(&a.name)),
                intensity: a.intensity,
                arrival: a.arrival,
                // P2P Quake relays warnings only: each of its areas is under the warning.
                warning: true,
                arrived: a.arrived(),
            })
            .collect();
        let max_intensity = areas.iter().filter_map(|a| a.intensity).max_by_key(|r| (r.strongest_known(), r.upper.is_none()));
        EewReport {
            source: SourceId::P2pquake,
            event_id: eew.event_id.clone(),
            serial: eew.serial,
            announced: eew.issued,
            origin: eew.origin,
            hypocentre: Hypocentre {
                name: eew.hypocentre.clone(),
                epicentre: eew.epicentre,
                depth_km: eew.depth_km.filter(|_| !eew.is_assumption),
                magnitude: eew.magnitude.filter(|_| !eew.is_assumption),
            },
            warning: !eew.cancelled,
            final_report: false,
            cancelled: eew.cancelled,
            training: eew.test,
            assumption: eew.is_assumption,
            max_intensity,
            areas,
        }
    }
}

/// What the early warnings of an earthquake say, as of the latest report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EewState {
    pub serial: u32,
    pub announced: Option<Time>,
    /// The latest report is a warning.
    pub warning: bool,
    /// Some report of the earthquake was a warning.
    pub ever_warning: bool,
    pub final_report: bool,
    pub cancelled: bool,
    pub training: bool,
    pub assumption: bool,
    pub max_intensity: Option<IntensityRange>,
    pub areas: Vec<EewArea>,
    pub sources: Vec<SourceId>,
    /// When the latest report was received.
    pub received: Time,
}

/// The kind of an earthquake report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    /// 震度速報: intensities of areas, before the hypocentre is known.
    Intensities,
    /// 震源に関する情報.
    Hypocentre,
    /// 震源・震度に関する情報.
    HypocentreAndIntensities,
    /// 各地の震度に関する情報.
    Detail,
    /// 遠地地震に関する情報.
    Foreign,
    Other,
}

impl ReportKind {
    /// How complete the kind is: a later report of a less complete kind does not replace a more
    /// complete one (P2P Quake may deliver 震度速報 after 各地の震度).
    fn rank(&self) -> u8 {
        match self {
            ReportKind::Other => 0,
            ReportKind::Intensities => 1,
            ReportKind::Hypocentre => 2,
            ReportKind::HypocentreAndIntensities => 3,
            ReportKind::Foreign => 3,
            ReportKind::Detail => 4,
        }
    }
}

/// Whether a tsunami is expected, as an earthquake report says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TsunamiOutlook {
    None,
    /// Slight changes of the sea level, no damage expected.
    Slight,
    Checking,
    Advisory,
    Warning,
    Unknown,
}

/// An intensity observed at a station or in an area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub prefecture: Option<Prefecture>,
    pub name: String,
    pub intensity: Intensity,
    pub is_area: bool,
    /// 5弱 or more estimated, not received.
    pub estimated: bool,
}

/// One earthquake report, from either relay.
#[derive(Debug, Clone, PartialEq)]
pub struct QuakeReport {
    pub source: SourceId,
    pub event_id: Option<String>,
    pub kind: ReportKind,
    pub issued: Option<Time>,
    pub origin: Option<Time>,
    pub hypocentre: Hypocentre,
    pub max_intensity: Option<Intensity>,
    pub tsunami: TsunamiOutlook,
    /// `None` when the source gives no observations (the Wolfx list), which keeps those of
    /// another source.
    pub observations: Option<Vec<Observation>>,
    pub comment: Option<String>,
}

impl QuakeReport {
    pub fn from_p2pquake(quake: &p2pquake::Earthquake) -> QuakeReport {
        use p2pquake::{DomesticTsunami, IssueKind};
        let kind = match quake.kind {
            IssueKind::ScalePrompt => ReportKind::Intensities,
            IssueKind::Destination => ReportKind::Hypocentre,
            IssueKind::ScaleAndDestination => ReportKind::HypocentreAndIntensities,
            IssueKind::DetailScale => ReportKind::Detail,
            IssueKind::Foreign => ReportKind::Foreign,
            IssueKind::Other(_) => ReportKind::Other,
        };
        let tsunami = match quake.domestic_tsunami {
            DomesticTsunami::None => TsunamiOutlook::None,
            DomesticTsunami::NonEffective => TsunamiOutlook::Slight,
            DomesticTsunami::Checking => TsunamiOutlook::Checking,
            DomesticTsunami::Watch => TsunamiOutlook::Advisory,
            DomesticTsunami::Warning => TsunamiOutlook::Warning,
            DomesticTsunami::Unknown | DomesticTsunami::Other(_) => TsunamiOutlook::Unknown,
        };
        let observations = quake
            .points
            .iter()
            .map(|p| Observation {
                prefecture: japan::Prefecture::from_name(&p.prefecture).or_else(|| japan::Prefecture::prefix_of(&p.prefecture)),
                name: p.name.clone(),
                intensity: p.intensity,
                is_area: p.is_area,
                estimated: p.estimated,
            })
            .collect();
        QuakeReport {
            source: SourceId::P2pquake,
            event_id: None,
            kind,
            issued: quake.issued,
            origin: quake.origin,
            hypocentre: Hypocentre { name: quake.hypocentre.clone(), epicentre: quake.epicentre, depth_km: quake.depth_km, magnitude: quake.magnitude },
            max_intensity: quake.max_intensity,
            tsunami,
            observations: Some(observations),
            comment: quake.comment.clone(),
        }
    }

    pub fn from_wolfx(entry: &wolfx::EarthquakeEntry) -> QuakeReport {
        let kind = match entry.title.as_deref() {
            Some("震度速報") => ReportKind::Intensities,
            Some("震源に関する情報") => ReportKind::Hypocentre,
            Some("震源・震度情報") | Some("震源・震度に関する情報") => ReportKind::HypocentreAndIntensities,
            Some("遠地地震に関する情報") => ReportKind::Foreign,
            _ => ReportKind::Other,
        };
        let tsunami = match entry.info.as_deref() {
            Some(info) if info.contains("津波の心配はありません") => TsunamiOutlook::None,
            Some(info) if info.contains("若干の海面変動") => TsunamiOutlook::Slight,
            Some(info) if info.contains("津波警報") => TsunamiOutlook::Warning,
            Some(info) if info.contains("津波注意報") => TsunamiOutlook::Advisory,
            _ => TsunamiOutlook::Unknown,
        };
        QuakeReport {
            source: SourceId::Wolfx,
            event_id: entry.event_id.clone(),
            kind,
            // The list has no time of issue; the report is as recent as the list.
            issued: None,
            origin: entry.origin,
            hypocentre: Hypocentre { name: entry.location.clone(), epicentre: entry.epicentre, depth_km: entry.depth_km, magnitude: entry.magnitude },
            max_intensity: entry.max_intensity,
            tsunami,
            observations: None,
            comment: entry.info.clone(),
        }
    }
}

/// What the reports of an earthquake say, as of the most complete one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportState {
    pub kind: ReportKind,
    pub issued: Option<Time>,
    pub max_intensity: Option<Intensity>,
    pub tsunami: TsunamiOutlook,
    pub comment: Option<String>,
    pub sources: Vec<SourceId>,
    pub received: Time,
}

/// The strongest intensity in one prefecture, observed or (`forecast`) expected.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrefectureIntensity {
    pub prefecture: Prefecture,
    pub intensity: Intensity,
    pub forecast: bool,
}

/// One earthquake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarthquakeEvent {
    /// The key of the event in this core: the JMA event ID when the first report had one.
    pub key: String,
    pub event_id: Option<String>,
    pub origin: Option<Time>,
    pub hypocentre: Hypocentre,
    pub eew: Option<EewState>,
    pub report: Option<ReportState>,
    pub prefectures: Vec<PrefectureIntensity>,
    pub local: Option<LocalImpact>,
    /// The highest level the event has alerted at.
    pub level: AlertLevel,
    pub first_seen: Time,
    pub updated: Time,
    /// The words for the lists, in the language of the snapshot.
    pub text: EventText,
}

/// An event with its observations, for its own screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDetail {
    pub event: EarthquakeEvent,
    pub observations: Vec<Observation>,
    /// The headline, the sentences and the advice, as an alert would word them now.
    pub title: String,
    pub body: String,
    pub actions: Option<String>,
}

impl EarthquakeEvent {
    /// The longest an early warning and a report of the same earthquake may differ in origin
    /// time to be joined (a warning estimates the origin, a list gives it to the minute).
    pub const SAME_ORIGIN_MS: i64 = 90_000;
    /// The farthest their epicentres may be apart to be joined.
    pub const SAME_EPICENTRE_KM: f64 = 150.0;

    pub fn new(key: String, now: Time) -> EarthquakeEvent {
        EarthquakeEvent {
            key,
            event_id: None,
            origin: None,
            hypocentre: Hypocentre::default(),
            eew: None,
            report: None,
            prefectures: Vec::new(),
            local: None,
            level: AlertLevel::None,
            first_seen: now,
            updated: now,
            text: EventText::default(),
        }
    }

    /// How closely a report without a shared event ID matches this earthquake: `None` when it
    /// is not of it, otherwise the difference of the origin times in ms (smaller is closer).
    ///
    /// The origins must be within `SAME_ORIGIN_MS`. With both epicentres known, they must be
    /// within `SAME_EPICENTRE_KM`. A report without an epicentre (震度速報) must have observed
    /// shaking in a prefecture this earthquake is known to concern (its warning's areas, its
    /// observations, or the prefecture of its epicentre); without that, it is not taken for this
    /// earthquake, so that a report of another earthquake never silences an active warning.
    pub fn closeness(&self, origin: Option<Time>, epicentre: Option<&Coordinate>, prefectures: &[Prefecture]) -> Option<i64> {
        let (Some(mine), Some(theirs)) = (self.origin, origin) else { return None };
        let difference = (mine.0 - theirs.0).abs();
        if difference > Self::SAME_ORIGIN_MS {
            return None;
        }
        match (self.hypocentre.epicentre.as_ref(), epicentre) {
            (Some(a), Some(b)) => (a.distance_km(b) <= Self::SAME_EPICENTRE_KM).then_some(difference),
            _ => {
                let concerned = self.concerned_prefectures();
                let overlap = !prefectures.is_empty() && prefectures.iter().any(|p| concerned.contains(p));
                overlap.then_some(difference)
            }
        }
    }

    /// The prefectures this earthquake is known to concern.
    pub fn concerned_prefectures(&self) -> Vec<Prefecture> {
        let mut list: Vec<Prefecture> = self.prefectures.iter().map(|p| p.prefecture).collect();
        if let Some(eew) = &self.eew {
            list.extend(eew.areas.iter().filter_map(|a| a.prefecture));
        }
        if let Some(epicentre) = &self.hypocentre.epicentre
            && let Some(p) = japan::Boundaries::built_in().locate_near(epicentre, Self::EPICENTRE_PREFECTURE_KM)
        {
            list.push(p);
        }
        list.sort();
        list.dedup();
        list
    }

    /// How far off its coast an epicentre still concerns a prefecture, for joining reports.
    pub const EPICENTRE_PREFECTURE_KM: f64 = 100.0;

    /// Takes an early warning report. Returns whether it was new: a report of the same serial
    /// from the other relay adds its source and fills gaps, and an older serial is ignored.
    pub fn apply_eew(&mut self, report: &EewReport, now: Time) -> bool {
        if self.event_id.is_none() {
            self.event_id = Some(report.event_id.clone());
        }
        let newer = match &self.eew {
            None => true,
            Some(state) => report.serial > state.serial || (report.cancelled && !state.cancelled),
        };
        if newer {
            let ever_warning = self.eew.as_ref().is_some_and(|s| s.ever_warning) || report.warning;
            let mut sources = vec![report.source];
            if let Some(state) = &self.eew
                && state.serial == report.serial
            {
                sources.extend(state.sources.iter().copied().filter(|s| *s != report.source));
            }
            let previous = self.eew.take();
            // A cancellation carries no earthquake: keep what was known.
            let (max_intensity, areas) = if report.cancelled && report.areas.is_empty() {
                previous.as_ref().map(|p| (p.max_intensity, p.areas.clone())).unwrap_or_default()
            } else {
                (report.max_intensity, report.areas.clone())
            };
            self.eew = Some(EewState {
                serial: report.serial,
                announced: report.announced,
                warning: report.warning,
                ever_warning,
                final_report: report.final_report,
                cancelled: report.cancelled,
                training: report.training,
                assumption: report.assumption,
                max_intensity,
                areas,
                sources,
                received: now,
            });
            // A warning's later hypocentre replaces the earlier one, unless a report has one.
            if self.report.is_none() && !report.cancelled {
                let mut hypocentre = report.hypocentre.clone();
                hypocentre.fill(&self.hypocentre);
                self.hypocentre = hypocentre;
                if report.origin.is_some() {
                    self.origin = report.origin;
                }
            }
        } else if let Some(state) = self.eew.as_mut() {
            if state.serial != report.serial {
                return false;
            }
            if !state.sources.contains(&report.source) {
                state.sources.push(report.source);
            }
            // The same report from the other relay: take the areas it has and this lacks, such
            // as the forecast areas Wolfx relays and P2P Quake does not.
            let mut changed = false;
            for area in &report.areas {
                match state.areas.iter_mut().find(|a| a.name == area.name) {
                    Some(existing) => {
                        if existing.arrival.is_none() && area.arrival.is_some() {
                            existing.arrival = area.arrival;
                            changed = true;
                        }
                        if existing.intensity.is_none() && area.intensity.is_some() {
                            existing.intensity = area.intensity;
                            changed = true;
                        }
                    }
                    None => {
                        state.areas.push(area.clone());
                        changed = true;
                    }
                }
            }
            if state.max_intensity.is_none() {
                state.max_intensity = report.max_intensity;
            }
            state.final_report |= report.final_report;
            if self.report.is_none() {
                self.hypocentre.fill(&report.hypocentre);
            }
            if !changed {
                return false;
            }
        }
        self.updated = now;
        self.refresh_prefectures(None);
        true
    }

    /// Takes an earthquake report; `observations` replaces the stored observations when given.
    /// Returns whether it changed the event.
    pub fn apply_report(&mut self, report: &QuakeReport, observations: &mut Vec<Observation>, now: Time) -> bool {
        if self.event_id.is_none() {
            self.event_id.clone_from(&report.event_id);
        }
        let replaces = match &self.report {
            None => true,
            Some(state) => {
                let rank = (report.kind.rank(), report.issued.map(|t| t.0).unwrap_or(0));
                let mine = (state.kind.rank(), state.issued.map(|t| t.0).unwrap_or(0));
                // The Wolfx list has no time of issue: it replaces only a report of lower rank,
                // or one of the same rank whose values it changes (a correction).
                if report.issued.is_none() {
                    report.kind.rank() > state.kind.rank()
                        || (report.kind.rank() == state.kind.rank() && (report.max_intensity != state.max_intensity || report.hypocentre != self.hypocentre))
                } else {
                    rank >= mine
                }
            }
        };
        if !replaces {
            let mut changed = false;
            if let Some(state) = self.report.as_mut()
                && !state.sources.contains(&report.source)
            {
                state.sources.push(report.source);
                changed = true;
            }
            let before = self.hypocentre.clone();
            self.hypocentre.fill(&report.hypocentre);
            changed |= before != self.hypocentre;
            if self.event_id.is_none() && report.event_id.is_some() {
                self.event_id.clone_from(&report.event_id);
                changed = true;
            }
            if changed {
                self.updated = now;
            }
            return changed;
        }
        let mut sources = vec![report.source];
        if let Some(state) = &self.report {
            sources.extend(state.sources.iter().copied().filter(|s| *s != report.source));
        }
        let new_state = ReportState {
            kind: report.kind,
            issued: report.issued.or(self.report.as_ref().and_then(|r| r.issued)),
            max_intensity: report.max_intensity.or(self.report.as_ref().and_then(|r| r.max_intensity)),
            tsunami: if report.tsunami == TsunamiOutlook::Unknown {
                self.report.as_ref().map(|r| r.tsunami).unwrap_or(TsunamiOutlook::Unknown)
            } else {
                report.tsunami
            },
            comment: report.comment.clone().or_else(|| self.report.as_ref().and_then(|r| r.comment.clone())),
            sources,
            received: now,
        };
        let changed_values = self.report.as_ref().is_none_or(|r| {
            r.kind != new_state.kind || r.max_intensity != new_state.max_intensity || r.tsunami != new_state.tsunami || r.issued != new_state.issued
        });
        // A report's hypocentre is the determined one: it replaces the warning's.
        let mut hypocentre = report.hypocentre.clone();
        hypocentre.fill(&self.hypocentre);
        let moved = hypocentre != self.hypocentre;
        self.hypocentre = hypocentre;
        if report.origin.is_some() && (self.report.is_none() || report.kind.rank() >= ReportKind::Hypocentre.rank()) {
            self.origin = report.origin;
        }
        self.report = Some(new_state);
        let observed = if let Some(new) = &report.observations {
            if !new.is_empty() || observations.is_empty() {
                let changed = new != observations;
                *observations = new.clone();
                changed
            } else {
                false
            }
        } else {
            false
        };
        self.refresh_prefectures(Some(observations));
        if changed_values || moved || observed {
            self.updated = now;
            true
        } else {
            false
        }
    }

    /// The strongest intensity of each prefecture: observed when there are observations,
    /// otherwise forecast by the early warning.
    pub fn refresh_prefectures(&mut self, observations: Option<&[Observation]>) {
        let mut list: Vec<PrefectureIntensity> = Vec::new();
        let mut put = |prefecture: Prefecture, intensity: Intensity, forecast: bool| match list.iter_mut().find(|p| p.prefecture == prefecture) {
            Some(entry) => entry.intensity = entry.intensity.max(intensity),
            None => list.push(PrefectureIntensity { prefecture, intensity, forecast }),
        };
        match observations.filter(|o| !o.is_empty()) {
            Some(observations) => {
                for o in observations {
                    if let Some(p) = o.prefecture {
                        put(p, o.intensity, false);
                    }
                }
            }
            // Observations already shown stay until new ones come.
            None if self.prefectures.iter().any(|p| !p.forecast) => return,
            None => {
                if let Some(eew) = &self.eew {
                    for area in &eew.areas {
                        if let (Some(p), Some(range)) = (area.prefecture, area.intensity) {
                            put(p, range.strongest_known(), true);
                        }
                    }
                }
            }
        }
        list.sort_by(|a, b| b.intensity.cmp(&a.intensity).then(a.prefecture.cmp(&b.prefecture)));
        self.prefectures = list;
    }

    /// The highest intensity known: observed, or else forecast.
    pub fn max_intensity(&self) -> Option<Intensity> {
        self.report.as_ref().and_then(|r| r.max_intensity).or_else(|| self.eew.as_ref().and_then(|e| e.max_intensity).map(|r| r.strongest_known()))
    }

    /// The early warning is still current: within `ACTIVE_MS` of its latest report, not
    /// cancelled, and not yet followed by a report of the observed intensities.
    pub fn eew_active(&self, now: Time) -> bool {
        self.eew.as_ref().is_some_and(|e| !e.cancelled && now.0 - e.received.0 <= Self::EEW_ACTIVE_MS && self.report.is_none())
    }

    /// How long an early warning stays current after its latest report.
    pub const EEW_ACTIVE_MS: i64 = 180_000;
}

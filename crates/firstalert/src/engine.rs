//! The state machine of the core.

use crate::alert::{AlertLevel, Category, Notification, Policy};
use crate::connection::{Connection, ConnectionState, SourceId};
use crate::event::{EarthquakeEvent, EewReport, EventDetail, Observation, QuakeReport};
use crate::impact::LocalImpact;
use crate::protocol::{ConnectionStatus, Effect, Input, LogLevel, MapShape, PlaceSummary, PrefectureAreas, Response, Snapshot, Waves};
use crate::settings::{Language, PlaceSource, Settings};
use crate::store::{History, Notified, Store, StoredEvent};
use crate::tsunami::{SpecialEntry, TsunamiState};
use crate::words::{Composed, IntensityLabel, Phrasebook, phrasebook};
use jma::{Arrival, Intensity, Time};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;

/// What a request was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestKind {
    P2pHistory,
    Wolfx(wolfx::Endpoint),
}

/// The core. One instance serves one user; it is not shared between threads (the platform
/// calls it from one thread, or behind a lock).
pub struct Core {
    store: Store,
    settings: Settings,
    locale: String,
    started: bool,
    connections: Vec<Connection>,
    events: Vec<EarthquakeEvent>,
    observations: HashMap<String, Vec<Observation>>,
    tsunami: Option<TsunamiState>,
    special: Vec<SpecialEntry>,
    notified: BTreeMap<String, Notified>,
    seen: VecDeque<String>,
    requests: HashMap<u64, RequestKind>,
    next_request: u64,
    /// The servers' clock less the platform's, ms.
    clock_offset: i64,
    /// When Wolfx last gave the time (platform time).
    exact_offset_at: Option<Time>,
    dirty: bool,
    last_save: Time,
    changed: bool,
    effects: Vec<Effect>,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    /// How many earthquakes the history keeps.
    pub const HISTORY_LIMIT: usize = 300;
    /// How many P2P Quake ids are remembered to skip repeated deliveries.
    pub const SEEN_LIMIT: usize = 1000;
    /// How many messages each catch-up request asks P2P Quake for.
    pub const CATCH_UP_LIMIT: u32 = 30;
    /// The longest the history waits to be written after a change.
    pub const SAVE_DELAY_MS: i64 = 2_000;
    /// How long the time of Wolfx is preferred to that of P2P Quake.
    pub const OFFSET_FROM_WOLFX_MS: i64 = 10 * 60_000;
    /// The clock offset is taken from a server only when it is plausible.
    pub const MAX_CLOCK_OFFSET_MS: i64 = 24 * 3_600_000;

    pub fn new() -> Core {
        Core {
            store: Store { directory: None },
            settings: Settings::default(),
            locale: "ja".to_owned(),
            started: false,
            connections: SourceId::ALL.iter().map(|s| Connection::new(*s, false)).collect(),
            events: Vec::new(),
            observations: HashMap::new(),
            tsunami: None,
            special: Vec::new(),
            notified: BTreeMap::new(),
            seen: VecDeque::new(),
            requests: HashMap::new(),
            next_request: 1,
            clock_offset: 0,
            exact_offset_at: None,
            dirty: false,
            last_save: Time(0),
            changed: false,
            effects: Vec::new(),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn language(&self) -> Language {
        self.settings.language.unwrap_or_else(|| Language::of_locale(&self.locale))
    }

    fn words(&self) -> &'static dyn Phrasebook {
        phrasebook(self.language())
    }

    /// Handles a JSON input and answers with JSON, for the C interface. Input that is not valid
    /// is answered with an error, never a panic.
    pub fn handle_json(&mut self, json: &str) -> String {
        let response = match serde_json::from_str::<Input>(json) {
            Ok(input) => self.handle(input),
            Err(error) => Response { error: Some(format!("invalid input: {error}")), ..Response::default() },
        };
        serde_json::to_string(&response).unwrap_or_else(|e| format!("{{\"effects\":[],\"error\":\"cannot write the response: {e}\"}}"))
    }

    pub fn handle(&mut self, input: Input) -> Response {
        let now = input.now();
        self.changed = false;
        let mut response = Response::default();
        match input {
            Input::Start { data_dir, locale, .. } => self.start(now, data_dir, locale),
            Input::Opened { source, .. } => self.opened(now, source),
            Input::Received { source, text, .. } => self.received(now, source, &text),
            Input::Closed { source, reason, .. } => {
                if let Some(c) = self.connection(source) {
                    c.closed(now);
                }
                self.log(LogLevel::Warning, format!("{source:?}: closed: {reason}"));
                self.changed = true;
            }
            Input::Responded { id, status, body, .. } => self.responded(now, id, status, &body),
            Input::Tick { .. } => self.tick(now),
            Input::NetworkAvailable { .. } => {
                let effects: Vec<Effect> = self.connections.iter_mut().flat_map(|c| c.network_available(now)).collect();
                self.changed |= !effects.is_empty();
                self.effects.extend(effects);
            }
            Input::Configure { settings, .. } => self.configure(now, settings),
            Input::Located { coordinate, .. } => self.located(now, coordinate),
            Input::Locale { locale, .. } => {
                self.locale = locale;
                self.reword();
                self.changed = true;
            }
            Input::Snapshot { .. } => self.changed = true,
            Input::Detail { key, .. } => response.detail = self.detail(&key).map(Box::new),
            Input::Waves { .. } => response.waves = Some(self.waves(now)),
            Input::Map { .. } => response.map = Some(Self::map()),
            Input::Areas { prefecture, .. } => response.areas = japan::Prefecture::from_code(prefecture).map(|p| self.areas(p)),
            Input::Acknowledge { key, .. } => self.effects.push(Effect::Silence { key }),
            Input::Test { level, .. } => self.test(now, level),
            Input::Stop { .. } => {
                for c in self.connections.iter_mut() {
                    let effects = c.set_enabled(false, now);
                    self.effects.extend(effects);
                }
                self.started = false;
                self.save(now);
                self.changed = true;
            }
        }
        if self.started {
            self.schedule(now);
        }
        response.effects = std::mem::take(&mut self.effects);
        if self.changed {
            response.snapshot = Some(Box::new(self.snapshot(now)));
        }
        response
    }

    fn log(&mut self, level: LogLevel, message: String) {
        self.effects.push(Effect::Log { level, message });
    }

    fn connection(&mut self, source: SourceId) -> Option<&mut Connection> {
        self.connections.iter_mut().find(|c| c.source == source)
    }

    /// The time of the servers.
    fn server_now(&self, now: Time) -> Time {
        Time(now.0 + self.clock_offset)
    }

    fn request(&mut self, kind: RequestKind, url: String) {
        let id = self.next_request;
        self.next_request += 1;
        self.requests.insert(id, kind);
        self.effects.push(Effect::Request { id, url });
    }

    /// Asks both relays for what may have been missed while not connected.
    fn catch_up(&mut self, source: SourceId) {
        match source {
            SourceId::P2pquake => self.request(RequestKind::P2pHistory, p2pquake::Api::history_url(&[551, 552, 556], Self::CATCH_UP_LIMIT)),
            SourceId::Wolfx => {
                for endpoint in [wolfx::Endpoint::JmaEew, wolfx::Endpoint::JmaEarthquakeList, wolfx::Endpoint::JmaTsunami, wolfx::Endpoint::JmaSpecial] {
                    self.request(RequestKind::Wolfx(endpoint), endpoint.http_url());
                }
            }
        }
    }

    fn start(&mut self, now: Time, data_dir: Option<String>, locale: String) {
        self.store = Store { directory: data_dir.map(PathBuf::from) };
        self.locale = locale;
        match self.store.load_settings() {
            Ok(Some(settings)) => self.settings = Self::validated(settings),
            Ok(None) => {}
            Err(error) => self.log(LogLevel::Error, error),
        }
        match self.store.load_history() {
            Ok(Some(history)) => self.restore(history),
            Ok(None) => {}
            Err(error) => self.log(LogLevel::Error, error),
        }
        self.started = true;
        self.last_save = now;
        let sources = self.settings.sources;
        for source in SourceId::ALL {
            let enabled = match source {
                SourceId::Wolfx => sources.wolfx,
                SourceId::P2pquake => sources.p2pquake,
            };
            let mut connection = Connection::new(source, enabled);
            let effects = connection.start(now);
            self.effects.extend(effects);
            self.connections.retain(|c| c.source != source);
            self.connections.push(connection);
        }
        self.relocalise();
        self.changed = true;
    }

    fn validated(mut settings: Settings) -> Settings {
        settings.version = Settings::VERSION;
        settings.place = settings.place.map(|p| p.validated());
        settings
    }

    fn restore(&mut self, history: History) {
        for stored in history.earthquakes {
            self.observations.insert(stored.event.key.clone(), stored.observations);
            self.events.push(stored.event);
        }
        self.tsunami = history.tsunami;
        self.special = history.special;
        self.notified = history.notified;
        self.seen = history.seen.into_iter().collect();
    }

    fn save(&mut self, now: Time) {
        if !self.dirty {
            return;
        }
        let history = History {
            version: History::VERSION,
            earthquakes: self
                .events
                .iter()
                .map(|e| StoredEvent { event: e.clone(), observations: self.observations.get(&e.key).cloned().unwrap_or_default() })
                .collect(),
            tsunami: self.tsunami.clone(),
            special: self.special.clone(),
            notified: self.notified.clone(),
            seen: self.seen.iter().cloned().collect(),
        };
        if let Err(error) = self.store.save_history(&history) {
            self.log(LogLevel::Error, error);
        }
        self.dirty = false;
        self.last_save = now;
    }

    fn opened(&mut self, now: Time, source: SourceId) {
        if let Some(c) = self.connection(source) {
            let effects = c.opened(now);
            self.effects.extend(effects);
        }
        if source == SourceId::Wolfx {
            self.effects.push(Effect::Send { source, text: wolfx::Message::PING.to_owned() });
        }
        // Whatever was issued while the connection was not open (before the first one too: the
        // device may have started without a network) is asked for on every opening.
        self.catch_up(source);
        self.changed = true;
    }

    fn received(&mut self, now: Time, source: SourceId, text: &str) {
        let first = self.connections.iter().find(|c| c.source == source).is_some_and(|c| c.state != ConnectionState::Open);
        if let Some(c) = self.connection(source) {
            c.received(now);
        }
        self.changed |= first;
        match source {
            SourceId::Wolfx => match wolfx::Message::parse(text) {
                Ok(message) => self.wolfx(now, message),
                Err(error) => self.log(LogLevel::Warning, format!("Wolfx: unreadable message ({error}): {}", text.chars().take(200).collect::<String>())),
            },
            SourceId::P2pquake => match p2pquake::Message::parse(text) {
                Ok(message) => self.p2pquake(now, message, true),
                Err(error) => self.log(LogLevel::Warning, format!("P2P Quake: unreadable message ({error}): {}", text.chars().take(200).collect::<String>())),
            },
        }
    }

    fn wolfx(&mut self, now: Time, message: wolfx::Message) {
        match message {
            wolfx::Message::Heartbeat { timestamp, .. } | wolfx::Message::Pong { timestamp } => {
                if let Some(server) = timestamp {
                    self.learn_offset(now, Time(server), true);
                }
            }
            wolfx::Message::Eew(eew) => self.take_eew(self.server_now(now), EewReport::from_wolfx(&eew)),
            wolfx::Message::EarthquakeList(list) => self.take_list(self.server_now(now), &list),
            wolfx::Message::Tsunami(tsunami) => {
                let server = self.server_now(now);
                if let Some(state) = TsunamiState::from_wolfx(&tsunami, server) {
                    self.take_tsunami(server, state);
                }
            }
            wolfx::Message::Special(special) => self.take_special(self.server_now(now), &special),
            wolfx::Message::Other(_) => {}
        }
    }

    /// Learns the clock of the servers from a time a server gave at `now`. Wolfx's heartbeats
    /// are exact; the time P2P Quake received a message is a fraction of a second earlier than
    /// its delivery, and is used only while Wolfx has given none for `OFFSET_FROM_WOLFX_MS`.
    fn learn_offset(&mut self, now: Time, server: Time, exact: bool) {
        let offset = server.0 - now.0;
        if offset.abs() > Self::MAX_CLOCK_OFFSET_MS {
            return;
        }
        if exact {
            self.clock_offset = offset;
            self.exact_offset_at = Some(now);
        } else if self.exact_offset_at.is_none_or(|t| now.0 - t.0 > Self::OFFSET_FROM_WOLFX_MS) {
            self.clock_offset = offset;
        }
    }

    /// `live`: the message came on the WebSocket now, rather than in a response of `/history`.
    fn p2pquake(&mut self, now: Time, message: p2pquake::Message, live: bool) {
        if live && let Some(received) = message.received() {
            self.learn_offset(now, received, false);
        }
        let now = self.server_now(now);
        if let Some(id) = message.id() {
            if self.seen.iter().any(|s| s == id) {
                return;
            }
            self.seen.push_back(id.to_owned());
            while self.seen.len() > Self::SEEN_LIMIT {
                self.seen.pop_front();
            }
            self.dirty = true;
        }
        match message {
            p2pquake::Message::Eew(eew) => self.take_eew(now, EewReport::from_p2pquake(&eew)),
            p2pquake::Message::Earthquake(quake) => self.take_report(now, QuakeReport::from_p2pquake(&quake)),
            p2pquake::Message::Tsunami(tsunami) => self.take_tsunami(now, TsunamiState::from_p2pquake(&tsunami, now)),
            p2pquake::Message::EewDetection { .. } | p2pquake::Message::Other { .. } => {}
        }
    }

    fn responded(&mut self, now: Time, id: u64, status: u16, body: &str) {
        let Some(kind) = self.requests.remove(&id) else { return };
        if status != 200 {
            self.log(LogLevel::Warning, format!("{kind:?}: HTTP {status}: {}", body.chars().take(200).collect::<String>()));
            return;
        }
        match kind {
            RequestKind::P2pHistory => match p2pquake::Message::parse_list(body) {
                Ok((mut messages, errors)) => {
                    for error in errors {
                        self.log(LogLevel::Warning, format!("P2P Quake history: {error}"));
                    }
                    // Oldest first, so that later reports update earlier ones.
                    messages.reverse();
                    for message in messages {
                        self.p2pquake(now, message, false);
                    }
                }
                Err(error) => self.log(LogLevel::Warning, format!("P2P Quake history: {error}")),
            },
            RequestKind::Wolfx(endpoint) => match wolfx::Message::parse_http(endpoint, body) {
                Ok(message) => self.wolfx(now, message),
                Err(error) => self.log(LogLevel::Warning, format!("Wolfx {}: {error}", endpoint.name())),
            },
        }
    }

    /// The event of an early warning: only by its event ID, which every warning has. A warning
    /// never joins an earthquake by time alone.
    fn find_warning(&self, event_id: &str) -> Option<usize> {
        self.events.iter().position(|e| e.event_id.as_deref() == Some(event_id))
    }

    /// The event of a report: by its event ID when an event has it, else the closest event
    /// without a different event ID (`EarthquakeEvent::closeness`).
    fn find_report(
        &self,
        event_id: Option<&str>,
        origin: Option<Time>,
        epicentre: Option<&geodesy::Coordinate>,
        prefectures: &[japan::Prefecture],
    ) -> Option<usize> {
        if let Some(id) = event_id
            && let Some(index) = self.events.iter().position(|e| e.event_id.as_deref() == Some(id))
        {
            return Some(index);
        }
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.event_id.is_none() || event_id.is_none())
            .filter_map(|(i, e)| e.closeness(origin, epicentre, prefectures).map(|d| (i, d)))
            .min_by_key(|(_, d)| *d)
            .map(|(i, _)| i)
    }

    fn insert_event(&mut self, key: String, now: Time) -> usize {
        let key = if self.events.iter().any(|e| e.key == key) { format!("{key}-{}", now.0) } else { key };
        self.events.push(EarthquakeEvent::new(key, now));
        self.events.len() - 1
    }

    // The `take_*` functions take `now` in the servers' time (`server_now`), as every time of the
    // events and of the reports is.

    fn take_eew(&mut self, now: Time, report: EewReport) {
        if report.training && !self.settings.training {
            return;
        }
        let index = match self.find_warning(&report.event_id) {
            Some(index) => index,
            None => self.insert_event(report.event_id.clone(), now),
        };
        if !self.events[index].apply_eew(&report, now) {
            return;
        }
        self.update_event(index);
        let policy = Policy { settings: &self.settings, now };
        let level = policy.eew(&self.events[index]);
        let version = format!("{}{}", report.serial, if report.cancelled { "c" } else { "" });
        let event = self.events[index].clone();
        let composed = self.words().eew(&event, level, self.place_label().as_deref());
        let local = event.local.as_ref();
        let expected = local.and_then(|l| l.expected).filter(|e| *e >= Intensity::One);
        let intensity = expected.or_else(|| event.eew.as_ref().and_then(|e| e.max_intensity).map(|r| r.strongest_known()));
        let key = format!("eew:{}", event.key);
        if report.cancelled {
            self.effects.push(Effect::Silence { key: key.clone() });
        }
        let arrival = local.and_then(|l| l.arrival).filter(|_| level >= AlertLevel::Warning);
        self.notify(now, key, Category::Eew, level, composed, intensity, expected.is_some(), None, arrival, version, Some(event.key.clone()));
        self.trim();
    }

    fn take_report(&mut self, now: Time, report: QuakeReport) {
        let mut observed: Vec<japan::Prefecture> = report.observations.iter().flatten().filter_map(|o| o.prefecture).collect();
        observed.sort();
        observed.dedup();
        let index = match self.find_report(report.event_id.as_deref(), report.origin, report.hypocentre.epicentre.as_ref(), &observed) {
            Some(index) => index,
            None => {
                let key = report.event_id.clone().or_else(|| report.origin.map(|o| format!("q{}", o.0))).unwrap_or_else(|| format!("n{}", now.0));
                self.insert_event(key, now)
            }
        };
        let key = self.events[index].key.clone();
        let mut observations = self.observations.remove(&key).unwrap_or_default();
        let changed = self.events[index].apply_report(&report, &mut observations, now);
        self.observations.insert(key.clone(), observations);
        if !changed {
            return;
        }
        self.update_event(index);
        let policy = Policy { settings: &self.settings, now };
        let level = policy.report(&self.events[index]);
        let event = self.events[index].clone();
        let composed = self.words().report(&event, self.place_label().as_deref());
        let state = event.report.as_ref();
        let version =
            format!("{:?}{:?}{:?}{:?}", state.map(|r| r.kind), state.and_then(|r| r.max_intensity), event.hypocentre.magnitude, event.hypocentre.name);
        let local = event.local.as_ref().and_then(|l| l.observed);
        let intensity = local.or(state.and_then(|r| r.max_intensity));
        if level > AlertLevel::None && self.notified.contains_key(&format!("eew:{key}")) {
            // The report says what happened: the early warning is past.
            self.effects.push(Effect::Silence { key: format!("eew:{key}") });
            self.effects.push(Effect::Withdraw { key: format!("eew:{key}") });
        }
        self.notify(now, format!("quake:{key}"), Category::Earthquake, level, composed, intensity, local.is_some(), None, None, version, Some(key));
        self.trim();
    }

    fn take_list(&mut self, now: Time, list: &wolfx::EarthquakeList) {
        // The newest few are enough: the list is sent whole on every change.
        let mut entries: Vec<&wolfx::EarthquakeEntry> = list.entries.iter().take(10).collect();
        entries.reverse();
        for entry in entries {
            self.take_report(now, QuakeReport::from_wolfx(entry));
        }
    }

    fn take_tsunami(&mut self, now: Time, mut state: TsunamiState) {
        state.localise(self.settings.place.as_ref().and_then(|p| p.prefecture));
        let previous = self.tsunami.clone();
        if let Some(previous) = &previous {
            // An older report does not replace a newer one; the same report from the other
            // relay only adds its source.
            if let (Some(old), Some(new)) = (previous.issued, state.issued)
                && new < old
            {
                return;
            }
            if previous.same_report(&state) {
                if !previous.sources.contains(&state.sources[0]) {
                    let mut merged = previous.clone();
                    merged.sources.push(state.sources[0]);
                    self.tsunami = Some(merged);
                    self.dirty = true;
                }
                return;
            }
        }
        let raised = previous.as_ref().is_none_or(|p| p.raised_by(&state));
        let lifted_now = state.lifted && previous.as_ref().is_some_and(|p| p.in_force());
        self.tsunami = Some(state.clone());
        self.dirty = true;
        self.changed = true;
        if !(raised || lifted_now || previous.as_ref().is_some_and(|p| p.in_force() && state.in_force())) {
            return;
        }
        let policy = Policy { settings: &self.settings, now };
        let level = if state.lifted && !lifted_now { AlertLevel::None } else { policy.tsunami(&state) };
        let place = self.settings.place.as_ref().and_then(|p| p.prefecture);
        let composed = self.words().tsunami(&state, place);
        let version = format!("{:?}{:?}{}", state.issued, state.highest, state.areas.len());
        let key = "tsunami".to_owned();
        if lifted_now {
            self.effects.push(Effect::Silence { key: key.clone() });
        }
        self.notify(now, key.clone(), Category::Tsunami, level, composed, None, state.local.is_some(), state.local.or(state.highest), None, version, None);
        // Tsunami warnings rise and fall over hours and come again with the next earthquake: what
        // was notified is the level in force now, not the highest ever, so that a warning raised
        // again (or a new one after lifting) alerts again.
        if let Some(notified) = self.notified.get_mut(&key) {
            notified.level = if state.lifted { AlertLevel::None } else { level };
        }
    }

    fn take_special(&mut self, now: Time, reports: &wolfx::SpecialReports) {
        for report in &reports.reports {
            if self.special.iter().any(|s| s.id == report.id) {
                continue;
            }
            let entry = SpecialEntry::from_wolfx(report, now);
            self.special.insert(0, entry.clone());
            self.special.truncate(20);
            self.dirty = true;
            self.changed = true;
            let level = Policy { settings: &self.settings, now }.special(&entry);
            let composed = self.words().special(&entry);
            self.notify(now, format!("special:{}", entry.id), Category::Special, level, composed, None, false, None, None, entry.id.clone(), None);
        }
    }

    /// Recomputes what derives from an event: its meaning at the user's place and its words.
    fn update_event(&mut self, index: usize) {
        let key = self.events[index].key.clone();
        let observations = self.observations.get(&key).map(Vec::as_slice).unwrap_or(&[]);
        let local = LocalImpact::of(&self.events[index], observations, &self.settings);
        self.events[index].local = local;
        self.events[index].text = self.words().event_text(&self.events[index]);
        self.dirty = true;
        self.changed = true;
    }

    /// Notifies under `key` at `level`, unless that was notified already: an update that is no
    /// more severe updates the notification quietly, and one that changes nothing is skipped.
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &mut self,
        now: Time,
        key: String,
        category: Category,
        level: AlertLevel,
        composed: Composed,
        intensity: Option<Intensity>,
        local: bool,
        tsunami: Option<jma::TsunamiGrade>,
        arrival: Option<Time>,
        version: String,
        subject: Option<String>,
    ) {
        let previous = self.notified.get(&key);
        if level == AlertLevel::None && previous.is_none() {
            return;
        }
        if previous.is_some_and(|p| p.version == version) {
            return;
        }
        let alerting = previous.is_none_or(|p| level > p.level || (level >= AlertLevel::Warning && intensity > p.intensity));
        let shown_level =
            if level == AlertLevel::None { previous.map(|p| p.level).unwrap_or(AlertLevel::Information).min(AlertLevel::Information) } else { level };
        let mut notification = Notification {
            key: key.clone(),
            category,
            level: shown_level,
            alerting,
            title: composed.title,
            body: composed.body,
            actions: composed.actions,
            speech: composed.speech,
            intensity,
            intensity_label: intensity.map(|i| self.words().intensity_short(i)),
            local,
            tsunami,
            arrival,
            time: now,
            sound: None,
            sound_when_silenced: false,
            voice: false,
            vibration: None,
            flash: false,
            full_screen: false,
            ongoing: false,
            subject: subject.clone(),
        };
        notification.present(&self.settings);
        let highest = previous.map(|p| p.level).unwrap_or(AlertLevel::None).max(level);
        let strongest = previous.and_then(|p| p.intensity).max(intensity);
        self.notified.insert(key, Notified { level: highest, intensity: strongest, version });
        if let Some(subject) = subject
            && let Some(event) = self.events.iter_mut().find(|e| e.key == subject)
        {
            event.level = event.level.max(level);
        }
        self.effects.push(Effect::Notify { notification: Box::new(notification) });
        self.dirty = true;
        self.changed = true;
    }

    /// Keeps the newest earthquakes only.
    fn trim(&mut self) {
        self.events.sort_by_key(|e| std::cmp::Reverse(e.origin.unwrap_or(e.first_seen)));
        if self.events.len() > Self::HISTORY_LIMIT {
            for removed in self.events.drain(Self::HISTORY_LIMIT..) {
                self.observations.remove(&removed.key);
            }
        }
        // Notifications of earthquakes no longer kept are not needed to compare with.
        let keys: Vec<String> = self.notified.keys().cloned().collect();
        for key in keys {
            let subject = key.split_once(':').map(|(_, s)| s);
            let kept = match key.split_once(':').map(|(k, _)| k) {
                Some("eew") | Some("quake") => self.events.iter().any(|e| Some(e.key.as_str()) == subject),
                Some("special") => self.special.iter().any(|s| Some(s.id.as_str()) == subject),
                _ => true,
            };
            if !kept {
                self.notified.remove(&key);
            }
        }
    }

    fn tick(&mut self, now: Time) {
        let effects: Vec<Effect> = self.connections.iter_mut().flat_map(|c| c.tick(now)).collect();
        if !effects.is_empty() {
            self.changed = true;
        }
        self.effects.extend(effects);
        // An early warning that has just stopped being active changes the state.
        let server = self.server_now(now);
        if self.events.iter().any(|e| {
            e.eew.as_ref().is_some_and(|w| !w.cancelled && e.report.is_none() && (server.0 - w.received.0 - EarthquakeEvent::EEW_ACTIVE_MS).abs() < 2_000)
        }) {
            self.changed = true;
        }
        if self.dirty && now.0 - self.last_save.0 >= Self::SAVE_DELAY_MS {
            self.save(now);
        }
    }

    /// Asks for a tick when the next timer is due.
    fn schedule(&mut self, now: Time) {
        let mut wake: Vec<i64> = self.connections.iter().filter_map(|c| c.next_wake()).map(|t| t.0).collect();
        if self.dirty {
            wake.push(self.last_save.0 + Self::SAVE_DELAY_MS);
        }
        let server = self.server_now(now);
        for event in &self.events {
            if let Some(eew) = &event.eew
                && event.eew_active(server)
            {
                // The times of the events are the servers'; the wake is the platform's.
                wake.push(eew.received.0 + EarthquakeEvent::EEW_ACTIVE_MS - self.clock_offset);
            }
        }
        if let Some(at) = wake.into_iter().min() {
            self.effects.push(Effect::Wake { at: Time(at.max(now.0)) });
        }
    }

    fn configure(&mut self, now: Time, settings: Settings) {
        let settings = Self::validated(settings);
        let sources = settings.sources;
        let language_changed = settings.language != self.settings.language;
        self.settings = settings;
        if let Err(error) = self.store.save_settings(&self.settings) {
            self.log(LogLevel::Error, error);
        }
        if self.started {
            for source in SourceId::ALL {
                let enabled = match source {
                    SourceId::Wolfx => sources.wolfx,
                    SourceId::P2pquake => sources.p2pquake,
                };
                if let Some(c) = self.connection(source) {
                    let effects = c.set_enabled(enabled, now);
                    self.effects.extend(effects);
                }
            }
        }
        self.relocalise();
        if language_changed {
            self.reword();
        }
        self.changed = true;
    }

    fn located(&mut self, now: Time, coordinate: geodesy::Coordinate) {
        let Some(place) = &self.settings.place else { return };
        if place.source != PlaceSource::Device || geodesy::Coordinate::checked(coordinate.latitude, coordinate.longitude).is_none() {
            return;
        }
        // A move of less than 100 m changes nothing worth recomputing.
        if place.coordinate.distance_km(&coordinate) < 0.1 {
            return;
        }
        let area = place.area.clone();
        let mut moved = crate::settings::Place::at(PlaceSource::Device, coordinate);
        if moved.prefecture == place.prefecture {
            moved.area = area;
        }
        let mut settings = self.settings.clone();
        settings.place = Some(moved);
        self.configure(now, settings);
    }

    /// Recomputes the meaning of everything at the user's place, after the place changed.
    fn relocalise(&mut self) {
        for index in 0..self.events.len() {
            self.update_event(index);
        }
        let prefecture = self.settings.place.as_ref().and_then(|p| p.prefecture);
        if let Some(t) = self.tsunami.as_mut() {
            t.localise(prefecture);
        }
    }

    fn reword(&mut self) {
        let words = self.words();
        for event in self.events.iter_mut() {
            event.text = words.event_text(event);
        }
    }

    fn place_label(&self) -> Option<String> {
        self.settings.place.as_ref().map(|p| self.words().place(p))
    }

    fn test(&mut self, now: Time, level: AlertLevel) {
        let composed = self.words().test(level);
        let mut notification = Notification {
            key: "test".to_owned(),
            category: Category::Test,
            level,
            alerting: true,
            title: composed.title,
            body: composed.body,
            actions: composed.actions,
            speech: composed.speech,
            intensity: Some(Intensity::FiveLower),
            intensity_label: Some(self.words().intensity_short(Intensity::FiveLower)),
            local: true,
            tsunami: None,
            arrival: (level == AlertLevel::Emergency).then(|| Time(self.server_now(now).0 + 10_000)),
            time: now,
            sound: None,
            sound_when_silenced: false,
            voice: false,
            vibration: None,
            flash: false,
            full_screen: false,
            ongoing: false,
            subject: None,
        };
        notification.present(&self.settings);
        self.effects.push(Effect::Notify { notification: Box::new(notification) });
    }

    pub fn snapshot(&self, now: Time) -> Snapshot {
        let server = self.server_now(now);
        let mut active: Vec<(AlertLevel, String)> = self.events.iter().filter(|e| e.eew_active(server)).map(|e| (e.level, e.key.clone())).collect();
        if let Some(t) = &self.tsunami
            && t.in_force()
        {
            let level = Policy { settings: &self.settings, now: server }.tsunami(t).max(AlertLevel::Information);
            active.push((level, "tsunami".to_owned()));
        }
        active.sort_by_key(|a| std::cmp::Reverse(a.0));
        let level = active.first().map(|a| a.0).unwrap_or(AlertLevel::None);
        Snapshot {
            now,
            clock_offset_ms: self.clock_offset,
            language: self.language(),
            settings: self.settings.clone(),
            connections: self
                .connections
                .iter()
                .map(|c| ConnectionStatus {
                    source: c.source,
                    enabled: c.enabled,
                    state: c.state,
                    since: c.since,
                    last_message: c.last_message,
                    failures: c.failures,
                })
                .collect(),
            earthquakes: self.events.clone(),
            tsunami: self.tsunami.clone(),
            special: self.special.clone(),
            active: active.into_iter().map(|a| a.1).collect(),
            level,
            place: self.settings.place.as_ref().map(|p| PlaceSummary { place: p.clone(), label: self.words().place(p) }),
            intensities: self.intensity_labels(),
        }
    }

    /// The names and descriptions of the intensity classes in the current language.
    pub fn intensity_labels(&self) -> Vec<IntensityLabel> {
        self.words().intensity_labels()
    }

    fn detail(&self, key: &str) -> Option<EventDetail> {
        let event = self.events.iter().find(|e| e.key == key)?;
        let place = self.place_label();
        let composed =
            if event.report.is_some() { self.words().report(event, place.as_deref()) } else { self.words().eew(event, event.level, place.as_deref()) };
        Some(EventDetail {
            event: event.clone(),
            observations: self.observations.get(key).cloned().unwrap_or_default(),
            title: composed.title,
            body: composed.body,
            actions: composed.actions,
        })
    }

    fn waves(&self, now: Time) -> Vec<Waves> {
        let server = self.server_now(now);
        self.events
            .iter()
            .filter(|e| e.eew_active(server))
            .filter_map(|e| {
                let (origin, epicentre, depth) = (e.origin?, e.hypocentre.epicentre?, e.hypocentre.depth_km.unwrap_or(10.0));
                let elapsed = (server.0 - origin.0) as f64 / 1000.0;
                if !(0.0..=600.0).contains(&elapsed) {
                    return None;
                }
                let (p, s) = Arrival::radii_km(elapsed, depth);
                Some(Waves { key: e.key.clone(), epicentre, p_radius_km: p, s_radius_km: s })
            })
            .collect()
    }

    fn areas(&self, prefecture: japan::Prefecture) -> PrefectureAreas {
        PrefectureAreas {
            prefecture: prefecture.code(),
            name: self.words().prefecture(prefecture),
            representative: japan::Boundaries::built_in().shape(prefecture).map(|s| s.representative_point()),
            areas: japan::SeismicArea::of(prefecture).map(|a| a.name.to_owned()).collect(),
        }
    }

    fn map() -> Vec<MapShape> {
        japan::Boundaries::built_in()
            .shapes
            .iter()
            .map(|s| MapShape {
                prefecture: s.prefecture.code(),
                name: s.prefecture.name().to_owned(),
                rings: s.rings.iter().map(|r| r.points().iter().map(|p| [p.longitude, p.latitude]).collect()).collect(),
            })
            .collect()
    }
}

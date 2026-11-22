//! The behaviour of the core, as `docs/design.md` specifies it: which information alerts how
//! strongly, how the reports of both relays are joined, and how the connections recover.

use firstalert::alert::{AlertLevel, Notification, Pattern};
use firstalert::connection::{ConnectionState, SourceId};
use firstalert::settings::{Place, PlaceSource};
use firstalert::{Core, Effect, Input, Response, Settings};
use geodesy::Coordinate;
use jma::{Intensity, Time};

/// Kumamoto city.
const KUMAMOTO: Coordinate = Coordinate::new(32.80, 130.71);
/// Sapporo.
const SAPPORO: Coordinate = Coordinate::new(43.06, 141.35);

fn jst(text: &str) -> Time {
    Time::parse_jst(text).unwrap()
}

fn core_at(place: Option<Coordinate>, now: Time) -> Core {
    let mut core = Core::new();
    core.handle(Input::Start { now, data_dir: None, locale: "ja-JP".into() });
    let settings = Settings { place: place.map(|c| Place::at(PlaceSource::Point, c)), ..Settings::default() };
    core.handle(Input::Configure { now, settings });
    core
}

fn notifications(response: &Response) -> Vec<&Notification> {
    response.effects.iter().filter_map(|e| if let Effect::Notify { notification } = e { Some(notification.as_ref()) } else { None }).collect()
}

/// A Wolfx early warning for the 熊本 area.
fn wolfx_eew(serial: u32, warning: bool, magnitude: f64, cancel: bool, training: bool) -> String {
    format!(
        r#"{{"type":"jma_eew","Title":"緊急地震速報（{}）","Issue":{{"Source":"大阪","Status":"{}"}},"EventID":"20261201120000","Serial":{serial},
        "AnnouncedTime":"2026/12/01 12:00:{:02}","OriginTime":"2026/12/01 11:59:58","Hypocenter":"熊本県熊本地方","Latitude":32.75,"Longitude":130.76,
        "Magnitude":{magnitude},"Depth":10,"MaxIntensity":"6弱","WarnArea":[{{"Chiiki":"熊本県熊本","Shindo1":"6弱","Shindo2":"5強","Time":"120008","Type":"警報","Arrive":"未到達"}}],
        "isSea":false,"isTraining":{training},"isAssumption":false,"isWarn":{warning},"isFinal":false,"isCancel":{cancel},"OriginalText":""}}"#,
        if warning { "警報" } else { "予報" },
        if training { "訓練" } else { "通常" },
        2 + serial,
    )
}

#[test]
fn start_connects_to_both_relays_and_catches_up() {
    let mut core = Core::new();
    let response = core.handle(Input::Start { now: Time(1_000), data_dir: None, locale: "ja".into() });
    let connects: Vec<SourceId> = response.effects.iter().filter_map(|e| if let Effect::Connect { source, .. } = e { Some(*source) } else { None }).collect();
    assert_eq!(connects, vec![SourceId::Wolfx, SourceId::P2pquake]);
    assert!(response.snapshot.is_some());
    // What was issued before the connections opened is asked for when each opens.
    let requests =
        |r: &Response| r.effects.iter().filter_map(|e| if let Effect::Request { url, .. } = e { Some(url.clone()) } else { None }).collect::<Vec<_>>();
    let wolfx = core.handle(Input::Opened { now: Time(2_000), source: SourceId::Wolfx });
    assert_eq!(requests(&wolfx).len(), 4, "the Wolfx early warning, list, tsunami and special information");
    assert!(requests(&wolfx).iter().any(|u| u.ends_with("jma_eew.json")));
    let p2p = core.handle(Input::Opened { now: Time(2_000), source: SourceId::P2pquake });
    assert_eq!(requests(&p2p).len(), 1, "the P2P Quake history");
}

#[test]
fn a_warning_for_the_users_area_is_an_emergency() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let n = notifications(&response);
    assert_eq!(n.len(), 1);
    let alert = n[0];
    assert_eq!(alert.level, AlertLevel::Emergency);
    assert!(alert.alerting && alert.full_screen && alert.voice && alert.ongoing);
    assert_eq!(alert.sound, Some(Pattern::Emergency));
    assert!(alert.sound_when_silenced);
    assert!(alert.local);
    assert!(alert.intensity >= Some(Intensity::FiveUpper));
    // The forecast arrival of the area is used for the countdown.
    assert_eq!(alert.arrival, Some(jst("2026/12/01 12:00:08")));
    assert!(alert.title.contains("緊急地震速報"));
    assert!(alert.actions.as_deref().unwrap().contains("頭を守"));
    let snapshot = response.snapshot.unwrap();
    assert_eq!(snapshot.level, AlertLevel::Emergency);
    assert_eq!(snapshot.active, vec!["20261201120000".to_string()]);
}

#[test]
fn the_same_report_from_the_other_relay_does_not_alert_again() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let p2p = r#"{"code":556,"id":"p1","cancelled":false,"issue":{"time":"2026/12/01 12:00:03","eventId":"20261201120000","serial":"1"},
        "earthquake":{"originTime":"2026/12/01 11:59:58","arrivalTime":"2026/12/01 12:00:00","condition":"","hypocenter":{"name":"熊本県熊本地方","reduceName":"熊本県","latitude":32.75,"longitude":130.76,"depth":10,"magnitude":6.5}},
        "areas":[{"pref":"熊本","name":"熊本県熊本","scaleFrom":55,"scaleTo":60,"kindCode":"10","arrivalTime":"2026/12/01 12:00:08"}]}"#;
    let response = core.handle(Input::Received { now: Time(now.0 + 50), source: SourceId::P2pquake, text: p2p.into() });
    assert!(notifications(&response).is_empty());
    // The same message again (P2P Quake may repeat a delivery) is ignored outright.
    let again = core.handle(Input::Received { now: Time(now.0 + 60), source: SourceId::P2pquake, text: p2p.into() });
    assert!(notifications(&again).is_empty());
    assert_eq!(core.snapshot(now).earthquakes.len(), 1);
}

#[test]
fn an_update_alerts_again_only_when_more_severe() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let quieter = core.handle(Input::Received { now: Time(now.0 + 1000), source: SourceId::Wolfx, text: wolfx_eew(2, true, 6.4, false, false) });
    let n = notifications(&quieter);
    assert_eq!(n.len(), 1);
    assert!(!n[0].alerting, "an update of the same severity is shown quietly");
    assert_eq!(n[0].sound, None);
    assert!(!n[0].full_screen);
}

#[test]
fn a_cancellation_is_said_quietly_and_stops_the_alarm() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let response = core.handle(Input::Received { now: Time(now.0 + 2000), source: SourceId::Wolfx, text: wolfx_eew(2, true, 6.5, true, false) });
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Silence { key } if key == "eew:20261201120000")));
    let n = notifications(&response);
    assert_eq!(n[0].level, AlertLevel::Information);
    assert!(n[0].title.contains("取り消"));
    assert!(response.snapshot.unwrap().active.is_empty());
}

#[test]
fn a_warning_far_away_is_shown_quietly() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(SAPPORO), now);
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let n = notifications(&response);
    assert_eq!(n[0].level, AlertLevel::Information);
    assert_eq!(n[0].sound, None);
    assert!(!n[0].local);
}

#[test]
fn without_a_place_a_warning_alerts_with_sound() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(None, now);
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    assert_eq!(notifications(&response)[0].level, AlertLevel::Warning);
}

#[test]
fn a_small_forecast_far_away_does_not_notify() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(SAPPORO), now);
    let text = wolfx_eew(1, false, 3.5, false, false)
        .replace(r#""MaxIntensity":"6弱""#, r#""MaxIntensity":"2""#)
        .replace(r#""Shindo1":"6弱","Shindo2":"5強""#, r#""Shindo1":"2","Shindo2":"1""#);
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text });
    assert!(notifications(&response).is_empty());
    // It is still recorded.
    assert_eq!(core.snapshot(now).earthquakes.len(), 1);
}

#[test]
fn old_warnings_are_recorded_without_alerting() {
    let late = jst("2026/12/01 12:30:00");
    let mut core = core_at(Some(KUMAMOTO), late);
    let response = core.handle(Input::Received { now: late, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    assert!(notifications(&response).is_empty());
    assert_eq!(core.snapshot(late).earthquakes.len(), 1);
}

#[test]
fn training_is_ignored_unless_asked_for() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, true) });
    assert!(notifications(&response).is_empty());
    assert!(core.snapshot(now).earthquakes.is_empty());
    let mut settings = core.settings().clone();
    settings.training = true;
    core.handle(Input::Configure { now, settings });
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, true) });
    let n = notifications(&response);
    assert_eq!(n[0].level, AlertLevel::Information, "training never alerts as a real warning");
    assert!(n[0].title.contains("訓練"));
}

#[test]
fn reports_of_both_relays_join_the_warning() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let later = jst("2026/12/01 12:04:00");
    let p2p = r#"{"code":551,"id":"q1","issue":{"source":"気象庁","time":"2026/12/01 12:03:30","type":"DetailScale","correct":"None"},
        "earthquake":{"time":"2026/12/01 11:59:00","hypocenter":{"name":"熊本県熊本地方","latitude":32.7,"longitude":130.8,"depth":10,"magnitude":6.4},"maxScale":55,"domesticTsunami":"None","foreignTsunami":"Unknown"},
        "points":[{"pref":"熊本県","addr":"熊本中央区","isArea":false,"scale":55},{"pref":"福岡県","addr":"福岡中央区","isArea":false,"scale":30}],"comments":{"freeFormComment":""}}"#;
    let response = core.handle(Input::Received { now: later, source: SourceId::P2pquake, text: p2p.into() });
    let n = notifications(&response);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].level, AlertLevel::Warning, "6弱 observed in the user's prefecture");
    assert_eq!(n[0].intensity, Some(Intensity::SixLower));
    assert!(n[0].body.contains("津波の心配はありません"));
    // The early warning is past: its notification is withdrawn.
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Withdraw { key } if key == "eew:20261201120000")));
    let snapshot = response.snapshot.unwrap();
    assert_eq!(snapshot.earthquakes.len(), 1, "the report joins the warning's event");
    let event = &snapshot.earthquakes[0];
    assert_eq!(event.hypocentre.magnitude, Some(6.4));
    assert_eq!(event.prefectures[0].intensity, Intensity::SixLower);
    assert!(!event.prefectures[0].forecast);
    assert!(snapshot.active.is_empty());

    // The Wolfx list of the same earthquake adds its source and nothing else.
    let list = r#"{"type":"jma_eqlist","No1":{"Title":"震源・震度情報","EventID":"20261201120000","time":"2026/12/01 11:59","time_full":"2026/12/01 11:59:00",
        "location":"熊本県熊本地方","magnitude":"6.4","shindo":"6-","depth":"10km","latitude":"32.7","longitude":"130.8","info":"この地震による津波の心配はありません。"},"md5":"a"}"#;
    let response = core.handle(Input::Received { now: Time(later.0 + 5000), source: SourceId::Wolfx, text: list.into() });
    assert!(notifications(&response).is_empty());
    assert_eq!(core.snapshot(later).earthquakes.len(), 1);
    let detail = core.handle(Input::Detail { now: later, key: "20261201120000".into() }).detail.unwrap();
    assert_eq!(detail.observations.len(), 2);
}

#[test]
fn tsunami_warning_on_the_users_coast_is_an_emergency_and_lifting_is_said() {
    let now = jst("2026/12/01 12:10:00");
    let mut core = core_at(Some(KUMAMOTO), now);
    let warning = r#"{"code":552,"id":"t1","cancelled":false,"issue":{"source":"気象庁","time":"2026/12/01 12:09:00","type":"Focus"},"areas":[
        {"grade":"Warning","immediate":true,"name":"有明・八代海","firstHeight":{"condition":"ただちに津波来襲と予測"},"maxHeight":{"description":"３ｍ","value":3}},
        {"grade":"Watch","immediate":false,"name":"長崎県西方","maxHeight":{"description":"１ｍ","value":1}}]}"#;
    let response = core.handle(Input::Received { now, source: SourceId::P2pquake, text: warning.into() });
    let n = notifications(&response);
    assert_eq!(n[0].level, AlertLevel::Emergency);
    assert!(n[0].actions.as_deref().unwrap().contains("高い所"));
    assert_eq!(response.snapshot.unwrap().active, vec!["tsunami".to_string()]);
    let lifted = r#"{"code":552,"id":"t2","cancelled":true,"issue":{"source":"気象庁","time":"2026/12/01 15:00:00","type":"Focus"},"areas":[]}"#;
    let later = jst("2026/12/01 15:00:10");
    let response = core.handle(Input::Received { now: later, source: SourceId::P2pquake, text: lifted.into() });
    let n = notifications(&response);
    assert_eq!(n[0].level, AlertLevel::Information);
    assert!(n[0].title.contains("解除"));
    assert!(response.snapshot.unwrap().active.is_empty());
}

#[test]
fn wolfx_unknown_tsunami_state_is_not_taken_for_none() {
    let now = jst("2026/12/01 12:10:00");
    let mut core = core_at(Some(KUMAMOTO), now);
    let warning =
        r#"{"code":552,"id":"t1","cancelled":false,"issue":{"time":"2026/12/01 12:09:00","type":"Focus"},"areas":[{"grade":"Warning","name":"有明・八代海"}]}"#;
    core.handle(Input::Received { now, source: SourceId::P2pquake, text: warning.into() });
    let unknown = r#"{"type":"jma_tsunami","status":"unknown","trusted":false,"areas":[]}"#;
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: unknown.into() });
    assert!(notifications(&response).is_empty());
    assert_eq!(core.snapshot(now).active, vec!["tsunami".to_string()]);
}

#[test]
fn connections_back_off_and_recover() {
    let mut core = Core::new();
    core.handle(Input::Start { now: Time(0), data_dir: None, locale: "ja".into() });
    core.handle(Input::Opened { now: Time(100), source: SourceId::Wolfx });
    let closed = core.handle(Input::Closed { now: Time(1_000), source: SourceId::Wolfx, reason: "reset".into() });
    let wake = closed.effects.iter().find_map(|e| if let Effect::Wake { at } = e { Some(*at) } else { None }).unwrap();
    assert!(wake.0 > 1_000 && wake.0 <= 1_500, "the first retry is after half a second: {wake:?}");
    let retry = core.handle(Input::Tick { now: wake });
    assert!(retry.effects.iter().any(|e| matches!(e, Effect::Connect { source: SourceId::Wolfx, .. })));
    // Reopened after a gap: what was missed is asked for.
    let reopened = core.handle(Input::Opened { now: Time(wake.0 + 100), source: SourceId::Wolfx });
    assert!(reopened.effects.iter().any(|e| matches!(e, Effect::Request { .. })));
    let status = reopened.snapshot.unwrap().connections.into_iter().find(|c| c.source == SourceId::Wolfx).unwrap();
    assert_eq!(status.state, ConnectionState::Open);
}

#[test]
fn a_silent_wolfx_connection_is_replaced() {
    let mut core = Core::new();
    core.handle(Input::Start { now: Time(0), data_dir: None, locale: "ja".into() });
    core.handle(Input::Opened { now: Time(0), source: SourceId::Wolfx });
    let response = core.handle(Input::Tick { now: Time(120_000) });
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Disconnect { source: SourceId::Wolfx })));
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Connect { source: SourceId::Wolfx, .. })));
}

#[test]
fn wolfx_is_pinged_and_heartbeats_set_the_clock() {
    let mut core = Core::new();
    core.handle(Input::Start { now: Time(0), data_dir: None, locale: "ja".into() });
    let opened = core.handle(Input::Opened { now: Time(0), source: SourceId::Wolfx });
    assert!(opened.effects.iter().any(|e| matches!(e, Effect::Send { source: SourceId::Wolfx, text } if text == "ping")));
    let response =
        core.handle(Input::Received { now: Time(10_000), source: SourceId::Wolfx, text: r#"{"type":"heartbeat","ver":27,"id":"1","timestamp":12500}"#.into() });
    let _ = response;
    assert_eq!(core.snapshot(Time(10_000)).clock_offset_ms, 2_500);
}

#[test]
fn turning_a_relay_off_disconnects_it() {
    let mut core = Core::new();
    core.handle(Input::Start { now: Time(0), data_dir: None, locale: "ja".into() });
    core.handle(Input::Opened { now: Time(10), source: SourceId::P2pquake });
    let mut settings = core.settings().clone();
    settings.sources.p2pquake = false;
    let response = core.handle(Input::Configure { now: Time(20), settings });
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Disconnect { source: SourceId::P2pquake })));
    let status = response.snapshot.unwrap().connections.into_iter().find(|c| c.source == SourceId::P2pquake).unwrap();
    assert_eq!(status.state, ConnectionState::Disabled);
}

#[test]
fn history_survives_a_restart_without_alerting_again() {
    let directory = std::env::temp_dir().join(format!("firstalert-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let now = jst("2026/12/01 12:00:03");
    {
        let mut core = Core::new();
        core.handle(Input::Start { now, data_dir: Some(directory.to_string_lossy().into()), locale: "ja".into() });
        let settings = Settings { place: Some(Place::at(PlaceSource::Point, KUMAMOTO)), ..Settings::default() };
        core.handle(Input::Configure { now, settings });
        core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
        core.handle(Input::Stop { now });
    }
    let mut core = Core::new();
    let response = core.handle(Input::Start { now, data_dir: Some(directory.to_string_lossy().into()), locale: "ja".into() });
    let snapshot = response.snapshot.unwrap();
    assert_eq!(snapshot.earthquakes.len(), 1);
    assert!(snapshot.settings.place.is_some());
    let again = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    assert!(notifications(&again).is_empty());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn a_damaged_history_is_kept_aside() {
    let directory = std::env::temp_dir().join(format!("firstalert-damaged-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("history.json"), b"{ not json").unwrap();
    let mut core = Core::new();
    let response = core.handle(Input::Start { now: Time(0), data_dir: Some(directory.to_string_lossy().into()), locale: "ja".into() });
    assert!(response.effects.iter().any(|e| matches!(e, Effect::Log { .. })));
    assert!(directory.join("history.json.broken").exists());
    assert!(response.snapshot.unwrap().earthquakes.is_empty());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn json_interface() {
    let mut core = Core::new();
    let answer: serde_json::Value = serde_json::from_str(&core.handle_json(r#"{"type":"start","now":0,"data_dir":null,"locale":"en-US"}"#)).unwrap();
    assert_eq!(answer["snapshot"]["language"], "english");
    assert!(answer["effects"].as_array().unwrap().len() >= 2);
    let error: serde_json::Value = serde_json::from_str(&core.handle_json("not an input")).unwrap();
    assert!(error["error"].as_str().unwrap().starts_with("invalid input"));
    let map: serde_json::Value = serde_json::from_str(&core.handle_json(r#"{"type":"map","now":0}"#)).unwrap();
    assert_eq!(map["map"].as_array().unwrap().len(), 47);
}

#[test]
fn waves_of_an_active_warning() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let waves = core.handle(Input::Waves { now: jst("2026/12/01 12:00:08") }).waves.unwrap();
    assert_eq!(waves.len(), 1);
    // 10 s after the origin, 10 km deep: P has travelled 78 km, S 45 km.
    let s = waves[0].s_radius_km.unwrap();
    assert!((s - (45.0f64 * 45.0 - 100.0).sqrt()).abs() < 1e-6, "{s}");
    assert!(waves[0].p_radius_km.unwrap() > s);
}

#[test]
fn words_follow_the_language() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(Some(KUMAMOTO), now);
    let mut settings = core.settings().clone();
    settings.language = Some(firstalert::settings::Language::English);
    core.handle(Input::Configure { now, settings });
    let response = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let n = notifications(&response);
    assert!(n[0].title.starts_with("Earthquake Early Warning"), "{}", n[0].title);
    assert!(n[0].body.contains("Kumamoto"));
    let snapshot = response.snapshot.unwrap();
    assert_eq!(snapshot.intensities.len(), 10);
    assert_eq!(snapshot.intensities[5].name, "Shindo 5-");
}

#[test]
fn areas_of_a_prefecture() {
    let mut core = Core::new();
    let areas = core.handle(Input::Areas { now: Time(0), prefecture: 43 }).areas.unwrap();
    assert_eq!(areas.name, "熊本県");
    assert!(areas.areas.contains(&"熊本県熊本".to_string()));
    assert!(areas.representative.is_some());
    assert!(core.handle(Input::Areas { now: Time(0), prefecture: 99 }).areas.is_none());
}

fn p2p_tsunami(id: &str, issued: &str, cancelled: bool, grade: &str) -> String {
    let areas = if cancelled { String::new() } else { format!(r#"{{"grade":"{grade}","immediate":true,"name":"岩手県"}}"#) };
    format!(r#"{{"code":552,"id":"{id}","cancelled":{cancelled},"issue":{{"source":"気象庁","time":"{issued}","type":"Focus"}},"areas":[{areas}]}}"#)
}

#[test]
fn a_new_tsunami_warning_after_one_was_lifted_alerts_again() {
    let iwate = Coordinate::new(39.64, 141.95);
    let first = jst("2026/11/20 10:00:10");
    let mut core = core_at(Some(iwate), first);
    let r = core.handle(Input::Received { now: first, source: SourceId::P2pquake, text: p2p_tsunami("a", "2026/11/20 10:00:00", false, "Warning") });
    assert!(notifications(&r)[0].alerting);
    let lifted = jst("2026/11/20 14:00:10");
    core.handle(Input::Received { now: lifted, source: SourceId::P2pquake, text: p2p_tsunami("b", "2026/11/20 14:00:00", true, "") });
    let again = jst("2026/11/25 08:00:10");
    let r = core.handle(Input::Received { now: again, source: SourceId::P2pquake, text: p2p_tsunami("c", "2026/11/25 08:00:00", false, "Warning") });
    let n = notifications(&r);
    assert_eq!(n[0].level, AlertLevel::Emergency);
    assert!(n[0].alerting && n[0].full_screen, "the second episode alerts as the first did");
}

#[test]
fn a_tsunami_advisory_raised_to_a_warning_again_alerts() {
    let iwate = Coordinate::new(39.64, 141.95);
    let now = jst("2026/11/20 10:00:10");
    let mut core = core_at(Some(iwate), now);
    core.handle(Input::Received { now, source: SourceId::P2pquake, text: p2p_tsunami("a", "2026/11/20 10:00:00", false, "Warning") });
    core.handle(Input::Received { now, source: SourceId::P2pquake, text: p2p_tsunami("b", "2026/11/20 11:00:00", false, "Watch") });
    let r = core.handle(Input::Received { now, source: SourceId::P2pquake, text: p2p_tsunami("c", "2026/11/20 12:00:00", false, "Warning") });
    assert!(notifications(&r)[0].alerting);
}

#[test]
fn wolfx_expired_is_not_an_all_clear() {
    let iwate = Coordinate::new(39.64, 141.95);
    let now = jst("2026/11/20 10:00:10");
    let mut core = core_at(Some(iwate), now);
    core.handle(Input::Received { now, source: SourceId::P2pquake, text: p2p_tsunami("a", "2026/11/20 10:00:00", false, "Warning") });
    let expired = r#"{"type":"jma_tsunami","status":"expired","trusted":true,"report":{"time":"2026-11-20T01:00:00.000Z"},"areas":[]}"#;
    let r = core.handle(Input::Received { now, source: SourceId::Wolfx, text: expired.into() });
    assert!(notifications(&r).is_empty());
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Silence { .. })));
    assert_eq!(core.snapshot(now).active, vec!["tsunami".to_string()]);
}

#[test]
fn a_report_of_another_earthquake_does_not_silence_an_active_warning() {
    let noto = Coordinate::new(37.3, 136.9);
    let now = jst("2026/12/01 12:00:35");
    let mut core = core_at(Some(noto), now);
    let eew = r#"{"type":"jma_eew","Title":"緊急地震速報（警報）","Issue":{"Status":"通常"},"EventID":"20261201120030","Serial":1,"AnnouncedTime":"2026/12/01 12:00:34","OriginTime":"2026/12/01 12:00:30",
        "Hypocenter":"石川県能登地方","Latitude":37.3,"Longitude":137.0,"Magnitude":7.0,"Depth":10,"MaxIntensity":"6強",
        "WarnArea":[{"Chiiki":"石川県能登","Shindo1":"6強","Shindo2":"6弱","Time":"120040","Type":"警報","Arrive":"未到達"}],"isWarn":true}"#;
    let r = core.handle(Input::Received { now, source: SourceId::Wolfx, text: eew.into() });
    assert_eq!(notifications(&r)[0].level, AlertLevel::Emergency);
    // A 震度速報 of a small earthquake in Hokkaido, half a minute earlier.
    let prompt = r#"{"code":551,"id":"h","issue":{"source":"気象庁","time":"2026/12/01 12:01:30","type":"ScalePrompt","correct":"None"},
        "earthquake":{"time":"2026/12/01 12:00:00","hypocenter":{"name":"","latitude":-200,"longitude":-200,"depth":-1,"magnitude":-1},"maxScale":30,"domesticTsunami":"Checking"},
        "points":[{"pref":"北海道","addr":"石狩地方中部","isArea":true,"scale":30}]}"#;
    let later = jst("2026/12/01 12:01:31");
    let r = core.handle(Input::Received { now: later, source: SourceId::P2pquake, text: prompt.into() });
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Silence { key } | Effect::Withdraw { key } if key.starts_with("eew:"))));
    let snapshot = core.snapshot(later);
    assert_eq!(snapshot.earthquakes.len(), 2, "two earthquakes, not one");
    assert_eq!(snapshot.active, vec!["20261201120030".to_string()]);
    // The 震度速報 of the same earthquake, in its prefecture, joins it.
    let own = prompt.replace(r#""id":"h""#, r#""id":"n""#).replace("北海道", "石川県").replace("石狩地方中部", "石川県能登");
    core.handle(Input::Received { now: later, source: SourceId::P2pquake, text: own });
    assert_eq!(core.snapshot(later).earthquakes.len(), 2);
}

#[test]
fn a_device_clock_behind_the_servers_keeps_the_warning_active() {
    // The device is four minutes behind; Wolfx's heartbeat tells.
    let server = jst("2026/12/01 12:00:03");
    let device = Time(server.0 - 240_000);
    let mut core = core_at(Some(KUMAMOTO), device);
    core.handle(Input::Received { now: device, source: SourceId::Wolfx, text: format!(r#"{{"type":"heartbeat","timestamp":{}}}"#, server.0) });
    let r = core.handle(Input::Received { now: device, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    assert_eq!(notifications(&r)[0].level, AlertLevel::Emergency);
    let snapshot = r.snapshot.unwrap();
    assert_eq!(snapshot.level, AlertLevel::Emergency);
    assert_eq!(snapshot.active.len(), 1);
    assert_eq!(core.handle(Input::Waves { now: Time(device.0 + 5_000) }).waves.unwrap().len(), 1);
}

#[test]
fn a_device_clock_ahead_still_alerts_from_p2p_quake_alone() {
    let issued = "2026/12/01 12:00:03";
    let device = Time(jst(issued).0 + 240_000);
    let mut core = core_at(Some(KUMAMOTO), device);
    let mut settings = core.settings().clone();
    settings.sources.wolfx = false;
    core.handle(Input::Configure { now: device, settings });
    let p2p = format!(
        r#"{{"code":556,"id":"p","time":"{issued}.100","cancelled":false,"issue":{{"time":"{issued}","eventId":"20261201120000","serial":"1"}},
        "earthquake":{{"originTime":"2026/12/01 11:59:58","arrivalTime":"2026/12/01 12:00:00","condition":"","hypocenter":{{"name":"熊本県熊本地方","latitude":32.75,"longitude":130.76,"depth":10,"magnitude":6.5}}}},
        "areas":[{{"pref":"熊本","name":"熊本県熊本","scaleFrom":55,"scaleTo":60,"kindCode":"10","arrivalTime":"2026/12/01 12:00:08"}}]}}"#
    );
    let r = core.handle(Input::Received { now: device, source: SourceId::P2pquake, text: p2p });
    assert_eq!(notifications(&r)[0].level, AlertLevel::Emergency);
}

#[test]
fn a_forecast_after_a_warning_stays_a_warning_without_a_place() {
    let now = jst("2026/12/01 12:00:03");
    let mut core = core_at(None, now);
    core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(1, true, 6.5, false, false) });
    let r = core.handle(Input::Received { now, source: SourceId::Wolfx, text: wolfx_eew(2, false, 6.5, false, false) });
    assert_eq!(notifications(&r)[0].level, AlertLevel::Warning);
}

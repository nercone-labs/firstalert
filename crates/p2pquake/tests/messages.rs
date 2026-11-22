//! The messages of P2P Quake, against `json-api-v2.yaml` and messages received from the service.

use geodesy::Coordinate;
use jma::{Intensity, IntensityRange, Time, TsunamiGrade};
use p2pquake::{Api, DomesticTsunami, IssueKind, Message};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn earthquake_reports_received() {
    let (messages, errors) = Message::parse_list(&fixture("551.json")).unwrap();
    assert!(errors.is_empty());
    assert_eq!(messages.len(), 3);
    let Message::Earthquake(quake) = &messages[0] else { panic!() };
    assert_eq!(quake.id, "6b0233d6719739069125466f");
    assert_eq!(quake.kind, IssueKind::DetailScale);
    assert_eq!(quake.correction, None);
    assert_eq!(quake.issued, Time::parse_jst("2026/11/22 09:14:13"));
    assert_eq!(quake.origin, Time::parse_jst("2026/11/22 09:09:00"));
    assert_eq!(quake.hypocentre.as_deref(), Some("茨城県南部"));
    assert_eq!(quake.epicentre, Some(Coordinate::new(36.1, 139.9)));
    assert_eq!(quake.depth_km, Some(50.0));
    assert_eq!(quake.magnitude, Some(2.7));
    assert_eq!(quake.max_intensity, Some(Intensity::One));
    assert_eq!(quake.domestic_tsunami, DomesticTsunami::None);
    assert_eq!(quake.points.len(), 1);
    assert_eq!(quake.points[0].prefecture, "茨城県");
    assert_eq!(quake.points[0].name, "常陸大宮市北町");
    assert_eq!(quake.points[0].intensity, Intensity::One);
    assert!(!quake.points[0].is_area && !quake.points[0].estimated);
    assert_eq!(quake.comment, None);
    let Message::Earthquake(third) = &messages[2] else { panic!() };
    assert_eq!(third.points.len(), 4);
}

#[test]
fn scale_prompt_without_hypocentre() {
    let text = r#"{"code":551,"id":"x","time":"2026/11/22 10:01:00.1","issue":{"source":"気象庁","time":"2026/11/22 10:01:00","type":"ScalePrompt","correct":"None"},
        "earthquake":{"time":"2026/11/22 09:59:00","hypocenter":{"name":"","latitude":-200,"longitude":-200,"depth":-1,"magnitude":-1},"maxScale":46,"domesticTsunami":"Checking","foreignTsunami":"Unknown"},
        "points":[{"pref":"石川県","addr":"石川県能登","isArea":true,"scale":46},{"pref":"富山県","addr":"富山県東部","isArea":true,"scale":40},{"pref":"x","addr":"y","isArea":true,"scale":99}],
        "comments":{"freeFormComment":""}}"#;
    let Message::Earthquake(quake) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(quake.kind, IssueKind::ScalePrompt);
    assert_eq!(quake.epicentre, None);
    assert_eq!(quake.depth_km, None);
    assert_eq!(quake.magnitude, None);
    assert_eq!(quake.hypocentre, None);
    assert_eq!(quake.max_intensity, Some(Intensity::FiveLower));
    assert_eq!(quake.domestic_tsunami, DomesticTsunami::Checking);
    assert_eq!(quake.points.len(), 2, "a scale the specification does not list is dropped");
    assert!(quake.points[0].is_area && quake.points[0].estimated);
}

#[test]
fn very_shallow_is_depth_zero() {
    let text = r#"{"code":551,"id":"y","issue":{"type":"Destination","correct":"DestinationOnly"},"earthquake":{"time":"2026/01/01 00:00:00","hypocenter":{"depth":0,"latitude":35,"longitude":139,"magnitude":3.0},"maxScale":-1}}"#;
    let Message::Earthquake(quake) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(quake.depth_km, Some(0.0));
    assert_eq!(quake.max_intensity, None);
    assert_eq!(quake.correction.as_deref(), Some("DestinationOnly"));
    assert_eq!(quake.kind, IssueKind::Destination);
}

#[test]
fn warnings_received() {
    let (messages, _) = Message::parse_list(&fixture("556.json")).unwrap();
    let Message::Eew(first) = &messages[0] else { panic!() };
    assert_eq!(first.event_id, "20261119022225");
    assert_eq!(first.serial, 1);
    assert!(first.is_assumption);
    assert!(!first.cancelled && !first.test);
    assert_eq!(first.origin, Time::parse_jst("2026/11/19 02:22:24"));
    assert_eq!(first.detected, Time::parse_jst("2026/11/19 02:22:25"));
    assert_eq!(first.hypocentre_short.as_deref(), Some("熊本県"));
    assert_eq!(first.areas.len(), 4);
    let area = &first.areas[0];
    assert_eq!(area.region, "熊本");
    assert_eq!(area.name, "熊本県天草・芦北");
    assert_eq!(area.intensity, Some(IntensityRange::at_least(Intensity::FiveLower)));
    assert_eq!(area.kind_code.as_deref(), Some("19"));
    assert_eq!(area.arrival, Time::parse_jst("2026/11/19 02:22:28"));
    let Message::Eew(second) = &messages[1] else { panic!() };
    assert!(!second.is_assumption);
    assert_eq!(second.magnitude, Some(5.5));
    assert_eq!(second.areas[1].intensity, Some(IntensityRange::exactly(Intensity::Four)));
    assert!(second.areas[1].arrived());
    assert_eq!(second.areas[1].arrival, None);
    assert_eq!(second.areas[3].intensity, Some(IntensityRange { lower: Intensity::Three, upper: Some(Intensity::Four) }));
}

#[test]
fn the_example_of_the_specification() {
    // From json-api-v2.yaml: depth and scales carry a decimal point, only the whole part counts.
    let text = r#"{"id":"e","code":556,"cancelled":false,"earthquake":{"condition":"","hypocenter":{"name":"宗谷地方北部","reduceName":"北海道道北","latitude":44.9,"longitude":142.1,"depth":10.0,"magnitude":5.5},
        "originTime":"2022/08/11 00:53:00","arrivalTime":"2022/08/11 00:53:10"},"issue":{"time":"2022/08/11 00:53:24","eventId":"20220811005302","serial":"1"},
        "areas":[{"pref":"北海道道北","name":"上川地方北部","scaleFrom":45.0,"scaleTo":45.0,"kindCode":"11","arrivalTime":null}]}"#;
    let Message::Eew(eew) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(eew.depth_km, Some(10.0));
    assert_eq!(eew.areas[0].intensity, Some(IntensityRange::exactly(Intensity::FiveLower)));
    assert!(eew.areas[0].arrived());
}

#[test]
fn cancelled_warning_has_no_earthquake() {
    let text = r#"{"id":"c","code":556,"cancelled":true,"issue":{"time":"2026/01/01 00:00:10","eventId":"20260101000000","serial":"3"},"areas":[]}"#;
    let Message::Eew(eew) = Message::parse(text).unwrap() else { panic!() };
    assert!(eew.cancelled);
    assert_eq!(eew.serial, 3);
    assert_eq!(eew.epicentre, None);
    // Without an event ID, a warning cannot be matched to its earthquake.
    assert!(Message::parse(r#"{"id":"c","code":556,"issue":{"serial":"3"}}"#).is_err());
}

#[test]
fn tsunami_forecasts_received() {
    let (messages, _) = Message::parse_list(&fixture("552.json")).unwrap();
    let Message::Tsunami(lifted) = &messages[0] else { panic!() };
    assert!(lifted.cancelled && lifted.areas.is_empty());
    let Message::Tsunami(watch) = &messages[1] else { panic!() };
    assert!(!watch.cancelled);
    let area = &watch.areas[0];
    assert_eq!(area.name, "有明・八代海");
    assert_eq!(area.grade, TsunamiGrade::Advisory);
    assert!(area.immediate);
    assert_eq!(area.first_condition.as_deref(), Some("津波到達中と推測"));
    assert_eq!(area.max_height_text.as_deref(), Some("１ｍ"));
    assert_eq!(area.max_height_m, Some(1.0));
}

#[test]
fn tsunami_example_of_the_specification() {
    let text = r#"{"id":"t","code":552,"cancelled":false,"issue":{"source":"気象庁","time":"2019/06/18 22:24:00","type":"Focus"},"areas":[
        {"grade":"Warning","immediate":true,"name":"福島県","firstHeight":{"condition":"津波到達中と推測"},"maxHeight":{"description":"３ｍ","value":3}},
        {"grade":"Watch","immediate":false,"name":"青森県太平洋沿岸","firstHeight":{"arrivalTime":"2019/06/18 22:40:00"},"maxHeight":{"description":"１ｍ","value":1}},
        {"grade":"MajorWarning","name":"岩手県","maxHeight":{"description":"巨大"}}]}"#;
    let Message::Tsunami(tsunami) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(tsunami.areas[0].grade, TsunamiGrade::Warning);
    assert_eq!(tsunami.areas[1].first_arrival, Time::parse_jst("2019/06/18 22:40:00"));
    assert_eq!(tsunami.areas[2].grade, TsunamiGrade::MajorWarning);
    assert_eq!(tsunami.areas[2].max_height_m, None);
}

#[test]
fn detections_and_other_codes() {
    let (messages, _) = Message::parse_list(&fixture("554.json")).unwrap();
    assert!(matches!(&messages[0], Message::EewDetection { id, .. } if id == "6afdded67197390691254605"));
    let other = Message::parse(r#"{"code":561,"id":"u","area":250}"#).unwrap();
    assert_eq!(other, Message::Other { code: 561, id: Some("u".into()) });
    assert_eq!(other.id(), Some("u"));
    assert!(Message::parse(r#"{"id":"no code"}"#).is_err());
    assert!(Message::parse("{").is_err());
    assert!(Message::parse_list("{}").is_err());
}

#[test]
fn history_url() {
    assert_eq!(Api::history_url(&[551, 552], 20), "https://api.p2pquake.net/v2/history?codes=551&codes=552&limit=20");
    assert!(Api::history_url(&[556], 500).ends_with("limit=100"));
}

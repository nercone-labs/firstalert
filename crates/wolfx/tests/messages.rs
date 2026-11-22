//! The messages of Wolfx, against the documentation (https://wolfx.jp/docs/open-api,
//! https://wolfx.jp/docs/websocket) and messages received from the service (`fixtures/`).

use geodesy::Coordinate;
use jma::{Intensity, IntensityRange, Time, TsunamiGrade};
use wolfx::{Endpoint, Message, SpecialProduct};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn heartbeat_and_pong() {
    let heartbeat = Message::parse(r#"{"type":"heartbeat","ver":27,"id":"388797","timestamp":1795340190623}"#).unwrap();
    assert_eq!(heartbeat, Message::Heartbeat { id: Some("388797".into()), timestamp: Some(1_795_340_190_623) });
    // The documentation calls the timestamp a string.
    let heartbeat = Message::parse(r#"{"type":"heartbeat","timestamp":"1795340190623"}"#).unwrap();
    assert_eq!(heartbeat, Message::Heartbeat { id: None, timestamp: Some(1_795_340_190_623) });
    assert_eq!(Message::parse(r#"{"type":"pong","timestamp":1795340190633}"#).unwrap(), Message::Pong { timestamp: Some(1_795_340_190_633) });
}

#[test]
fn other_sources_and_errors() {
    assert_eq!(Message::parse(r#"{"type":"cenc_eew","ID":"1"}"#).unwrap(), Message::Other("cenc_eew".into()));
    assert!(Message::parse("not json").is_err());
    assert!(Message::parse("[1,2]").is_err());
    assert!(Message::parse(r#"{"no":"type"}"#).is_err());
    // An EEW without its identity cannot be told apart from another.
    assert!(Message::parse(r#"{"type":"jma_eew","Title":"緊急地震速報（予報）"}"#).is_err());
}

#[test]
fn received_forecast() {
    let Message::Eew(eew) = Message::parse_http(Endpoint::JmaEew, &fixture("jma_eew.json")).unwrap() else { panic!() };
    assert_eq!(eew.event_id, "20261122174303");
    assert_eq!(eew.serial, 4);
    assert_eq!(eew.title.as_deref(), Some("緊急地震速報（予報）"));
    assert_eq!(eew.status.as_deref(), Some("通常"));
    assert_eq!(eew.announced, Time::parse_jst("2026/11/22 17:43:49"));
    assert_eq!(eew.origin, Time::parse_jst("2026/11/22 17:42:53"));
    assert_eq!(eew.hypocentre.as_deref(), Some("石垣島近海"));
    assert_eq!(eew.epicentre, Some(Coordinate::new(24.6, 124.6)));
    assert_eq!(eew.magnitude, Some(3.7));
    assert_eq!(eew.depth_km, Some(60.0));
    assert_eq!(eew.max_intensity, Some(IntensityRange::exactly(Intensity::One)));
    assert!(eew.areas.is_empty());
    assert!(eew.is_sea && eew.is_final);
    assert!(!eew.is_warning && !eew.is_cancel && !eew.is_training && !eew.is_assumption);
}

#[test]
fn the_websocket_form_adds_a_type() {
    let mut text = fixture("jma_eew.json");
    text.insert_str(1, r#""type":"jma_eew","#);
    let Message::Eew(eew) = Message::parse(&text).unwrap() else { panic!() };
    assert_eq!(eew.serial, 4);
}

#[test]
fn warning_with_areas() {
    let text = r#"{"type":"jma_eew","Title":"緊急地震速報（警報）","EventID":"20261231235955","Serial":3,
        "AnnouncedTime":"2026/12/31 23:59:58","OriginTime":"2026/12/31 23:59:50","Hypocenter":"能登半島沖",
        "Latitude":37.5,"Longitude":137.2,"Magnitude":6.8,"Depth":10,"MaxIntensity":"6強","isWarn":true,
        "WarnArea":[
          {"Chiiki":"石川県能登","Shindo1":"6強","Shindo2":"5強","Time":"000005","Type":"警報","Arrive":"未到達"},
          {"Chiiki":"富山県東部","Shindo1":"5弱以上","Shindo2":"5弱","Time":"2027/01/01 00:00:12","Type":"警報","Arrive":"既に到達と予測"},
          {"Chiiki":"新潟県上越","Shindo1":"4","Shindo2":"3","Time":"//////","Type":"予報","Arrive":"未到達"},
          {"Shindo1":"4"}
        ]}"#;
    let Message::Eew(eew) = Message::parse(text).unwrap() else { panic!() };
    assert!(eew.is_warning);
    assert_eq!(eew.max_intensity, Some(IntensityRange::exactly(Intensity::SixUpper)));
    assert_eq!(eew.areas.len(), 3, "an area without a name is dropped");
    let noto = &eew.areas[0];
    assert_eq!(noto.intensity, Some(IntensityRange { lower: Intensity::FiveUpper, upper: Some(Intensity::SixUpper) }));
    // 00:00:05 is the next day of the announcement at 23:59:58.
    assert_eq!(noto.arrival, Time::parse_jst("2027/01/01 00:00:05"));
    assert!(noto.warning && !noto.arrived);
    let toyama = &eew.areas[1];
    assert_eq!(toyama.intensity, Some(IntensityRange::at_least(Intensity::FiveLower)));
    assert_eq!(toyama.arrival, Time::parse_jst("2027/01/01 00:00:12"));
    assert!(toyama.arrived);
    let joetsu = &eew.areas[2];
    assert!(!joetsu.warning);
    assert_eq!(joetsu.arrival, None);
}

#[test]
fn unknown_values_are_absent() {
    let text = r#"{"type":"jma_eew","EventID":"1","Serial":"2","Latitude":0,"Longitude":0,"Magnitude":0,"Magunitude":0,"Depth":null,"MaxIntensity":"不明","isAssumption":true}"#;
    let Message::Eew(eew) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(eew.serial, 2);
    assert_eq!(eew.epicentre, None);
    assert_eq!(eew.magnitude, None);
    assert_eq!(eew.depth_km, None);
    assert_eq!(eew.max_intensity, None);
    assert!(eew.is_assumption);
    // The misspelt field is read when the correct one is missing.
    let Message::Eew(eew) = Message::parse(r#"{"type":"jma_eew","EventID":"1","Serial":1,"Magunitude":4.2}"#).unwrap() else { panic!() };
    assert_eq!(eew.magnitude, Some(4.2));
}

#[test]
fn earthquake_list() {
    let Message::EarthquakeList(list) = Message::parse_http(Endpoint::JmaEarthquakeList, &fixture("jma_eqlist.json")).unwrap() else { panic!() };
    assert_eq!(list.entries.len(), 50);
    assert!(list.md5.is_some());
    assert!(list.entries.windows(2).all(|w| w[0].index < w[1].index));
    let first = &list.entries[0];
    assert_eq!(first.index, 1);
    assert_eq!(first.event_id.as_deref(), Some("20261122090925"));
    assert_eq!(first.origin, Time::parse_jst("2026/11/22 09:09:00"));
    assert_eq!(first.location.as_deref(), Some("茨城県南部"));
    assert_eq!(first.magnitude, Some(2.7));
    assert_eq!(first.max_intensity, Some(Intensity::One));
    assert_eq!(first.depth_km, Some(50.0));
    assert_eq!(first.epicentre, Some(Coordinate::new(36.1, 139.9)));
    assert_eq!(first.info.as_deref(), Some("この地震による津波の心配はありません。"));
}

#[test]
fn earthquake_list_odd_entries() {
    let text = r#"{"type":"jma_eqlist","No1":{"Title":"震度速報","EventID":"20261122100000","time_full":"2026/11/22 10:00:00",
        "location":"震源 調査中","magnitude":"不明","shindo":"5-","depth":"不明","latitude":"0.0","longitude":"0.0","info":""},
        "No2":{"Title":"震源・震度情報","time":"2026/11/21 08:00","location":"千葉県北西部","magnitude":"4.1","shindo":"3","depth":"ごく浅い","latitude":"35.6","longitude":"140.1"},
        "No3":"","md5":"x"}"#;
    let Message::EarthquakeList(list) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(list.entries.len(), 2);
    let prompt = &list.entries[0];
    assert_eq!(prompt.location, None);
    assert_eq!(prompt.magnitude, None);
    assert_eq!(prompt.max_intensity, Some(Intensity::FiveLower));
    assert_eq!(prompt.epicentre, None);
    assert_eq!(prompt.depth_km, None);
    assert_eq!(prompt.info, None);
    let shallow = &list.entries[1];
    assert_eq!(shallow.depth_km, Some(0.0));
    assert_eq!(shallow.origin, Time::parse_jst("2026/11/21 08:00"));
}

#[test]
fn tsunami_without_areas() {
    let Message::Tsunami(tsunami) = Message::parse_http(Endpoint::JmaTsunami, &fixture("jma_tsunami.json")).unwrap() else { panic!() };
    assert_eq!(tsunami.status.as_deref(), Some("expired"));
    assert!(!tsunami.is_current());
    assert!(tsunami.trusted);
    assert_eq!(tsunami.valid_until, Time::parse_rfc3339("2026-11-22T09:37:29.634Z"));
    assert_eq!(tsunami.report_time, Time::parse_rfc3339("2026-11-06T10:53:00.000Z"));
    assert_eq!(tsunami.event_ids, vec!["20261106193937".to_string()]);
    assert!(tsunami.areas.is_empty());
    assert!(tsunami.warning_comment.unwrap().contains("若干の海面変動"));
}

#[test]
fn tsunami_with_areas() {
    let text = r#"{"type":"jma_tsunami","status":"active","trusted":true,"areas":[
        {"code":"210","name":"岩手県","level":"major_warning","kind":"52","firstWave":{"time":"2026-12-01T05:30:00Z","condition":"ただちに津波来襲と予測"},"maxWave":{"height":null,"text":"巨大"}},
        {"code":"220","name":"宮城県","level":"warning","maxWave":{"height":3,"text":"３ｍ"}},
        {"code":"250","name":"福島県","level":"cleared"},
        {"name":"茨城県","level":"new_kind"}]}"#;
    let Message::Tsunami(tsunami) = Message::parse(text).unwrap() else { panic!() };
    assert!(tsunami.is_current());
    assert_eq!(tsunami.areas.len(), 4);
    assert_eq!(tsunami.areas[0].grade, Some(TsunamiGrade::MajorWarning));
    assert_eq!(tsunami.areas[0].first_wave.time, Time::parse_rfc3339("2026-12-01T05:30:00Z"));
    assert_eq!(tsunami.areas[0].first_wave.condition.as_deref(), Some("ただちに津波来襲と予測"));
    assert_eq!(tsunami.areas[0].max_wave.height_m, None);
    assert_eq!(tsunami.areas[0].max_wave.text.as_deref(), Some("巨大"));
    assert_eq!(tsunami.areas[1].grade, Some(TsunamiGrade::Warning));
    assert_eq!(tsunami.areas[1].max_wave.height_m, Some(3.0));
    assert_eq!(tsunami.areas[2].grade, None);
    // A level this version does not know is kept, as unknown, rather than dropped.
    assert_eq!(tsunami.areas[3].grade, Some(TsunamiGrade::Unknown));
}

#[test]
fn endpoints() {
    assert_eq!(Endpoint::JmaEew.http_url(), "https://api.wolfx.jp/jma_eew.json");
    assert_eq!(Endpoint::JmaTsunami.query(), "query_jmatsunami");
    let url = Endpoint::combined_websocket_url();
    assert!(url.starts_with("wss://ws-api.wolfx.jp/all_eew?exclude="));
    for kept in ["jma_eew", "jma_eqlist", "jma_tsunami", "jma_special_eq"] {
        assert!(!url.contains(kept), "{url}");
    }
}

#[test]
fn special_reports() {
    let text = r#"{"type":"jma_special_eq","reports":[
        {"id":"a","product":"VYSE50","time":"2026-12-01T06:00:00.000Z","status":"issued","eventIds":["20261201145500"],"kind":"南海トラフ地震臨時情報","code":"120","label":"巨大地震注意","title":"南海トラフ地震臨時情報（巨大地震注意）","text":"本文"},
        {"id":"b","product":"VYSE52","time":"2026-11-09T08:00:00.000Z","status":"issued","label":"定例解説","title":"南海トラフ地震関連解説情報"},
        {"id":"c","product":"VXXX99","status":"cancelled"},
        {"product":"VYSE60"}],"md5":"m"}"#;
    let Message::Special(special) = Message::parse(text).unwrap() else { panic!() };
    assert_eq!(special.reports.len(), 3, "a report without an id is dropped");
    let nankai = &special.reports[0];
    assert_eq!(nankai.product, SpecialProduct::NankaiTroughExtra);
    assert_eq!(nankai.label.as_deref(), Some("巨大地震注意"));
    assert_eq!(nankai.time, Time::parse_rfc3339("2026-12-01T06:00:00Z"));
    assert!(!nankai.cancelled);
    assert!(nankai.product.is_extraordinary());
    assert_eq!(special.reports[1].product, SpecialProduct::NankaiTroughRelated);
    assert!(!special.reports[1].product.is_extraordinary());
    assert_eq!(special.reports[2].product, SpecialProduct::Other("VXXX99".into()));
    assert!(special.reports[2].cancelled);
}

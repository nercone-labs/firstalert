use geodesy::Coordinate;
use japan::{Boundaries, BoundaryError, Prefecture, SeismicArea, TsunamiArea};

#[test]
fn forty_seven_prefectures_in_jis_order() {
    assert_eq!(Prefecture::all().count(), 47);
    assert_eq!(Prefecture::from_code(1).unwrap().name(), "北海道");
    assert_eq!(Prefecture::from_code(13).unwrap().name(), "東京都");
    assert_eq!(Prefecture::from_code(27).unwrap().name(), "大阪府");
    assert_eq!(Prefecture::from_code(47).unwrap().name(), "沖縄県");
    assert!(Prefecture::from_code(0).is_none());
    assert!(Prefecture::from_code(48).is_none());
}

#[test]
fn names_and_prefixes() {
    let tokyo = Prefecture::from_code(13).unwrap();
    assert_eq!(tokyo.short_name(), "東京");
    assert_eq!(Prefecture::from_name("東京"), Some(tokyo));
    assert_eq!(Prefecture::from_name("東京都"), Some(tokyo));
    assert_eq!(Prefecture::prefix_of("熊本県熊本"), Prefecture::from_code(43));
    assert_eq!(Prefecture::prefix_of("北海道道北"), Prefecture::from_code(1));
    assert_eq!(Prefecture::prefix_of("京都府南部"), Prefecture::from_code(26));
    // 東京湾内湾 begins with 東京, but 京都 is not a prefix of it.
    assert_eq!(Prefecture::prefix_of("東京湾内湾"), Some(tokyo));
    assert_eq!(Prefecture::prefix_of("有明・八代海"), None);
}

#[test]
fn seismic_areas_cover_every_prefecture() {
    let areas = SeismicArea::all();
    assert_eq!(areas.len(), 188);
    for prefecture in Prefecture::all() {
        assert!(SeismicArea::of(prefecture).next().is_some(), "{}", prefecture.name());
    }
    let area = SeismicArea::find("石狩地方中部").unwrap();
    assert_eq!(area.region, "北海道道央");
    assert_eq!(area.prefecture, Prefecture::from_code(1).unwrap());
}

#[test]
fn regions_of_the_warnings_name_their_prefecture() {
    assert_eq!(SeismicArea::prefecture_of_region("熊本"), Prefecture::from_code(43));
    assert_eq!(SeismicArea::prefecture_of_region("北海道道北"), Prefecture::from_code(1));
    assert_eq!(SeismicArea::prefecture_of_region("伊豆諸島"), Prefecture::from_code(13));
    assert_eq!(SeismicArea::prefecture_of_region("奄美群島"), Prefecture::from_code(46));
    assert_eq!(SeismicArea::prefecture_of("上川地方北部"), Prefecture::from_code(1));
    assert_eq!(SeismicArea::prefecture_of("鹿児島県薩摩"), Prefecture::from_code(46));
}

#[test]
fn tsunami_areas() {
    assert_eq!(TsunamiArea::all().len(), 66);
    let ariake = TsunamiArea::prefectures_of("有明・八代海");
    for code in [40, 41, 42, 43] {
        assert!(ariake.contains(&Prefecture::from_code(code).unwrap()));
    }
    assert_eq!(TsunamiArea::prefectures_of("伊豆諸島"), vec![Prefecture::from_code(13).unwrap()]);
    // An area not in the list falls back to the prefecture it is named after.
    assert_eq!(TsunamiArea::prefectures_of("福島県北部"), vec![Prefecture::from_code(7).unwrap()]);
}

#[test]
fn boundaries_locate_familiar_places() {
    let boundaries = Boundaries::built_in();
    assert_eq!(boundaries.shapes.len(), 47);
    let cases = [
        (35.6812, 139.7671, 13), // Tokyo station
        (34.7025, 135.4959, 27), // Osaka station
        (43.0687, 141.3508, 1),  // Sapporo station
        (26.2124, 127.6792, 47), // Naha
        (32.7898, 130.6880, 43), // Kumamoto station
        (38.2601, 140.8824, 4),  // Sendai station
    ];
    for (lat, lon, code) in cases {
        assert_eq!(boundaries.locate_near(&Coordinate::new(lat, lon), 5.0), Prefecture::from_code(code), "{lat},{lon}");
    }
    assert_eq!(boundaries.locate(&Coordinate::new(30.0, 145.0)), None);
}

#[test]
fn representative_points_lie_in_their_prefecture() {
    let boundaries = Boundaries::built_in();
    for shape in &boundaries.shapes {
        let point = shape.representative_point();
        assert_eq!(boundaries.locate_near(&point, 30.0), Some(shape.prefecture), "{}", shape.prefecture.name());
    }
}

#[test]
fn malformed_boundaries_are_errors() {
    assert_eq!(Boundaries::parse(b"NOPE"), Err(BoundaryError::Magic));
    assert_eq!(Boundaries::parse(b"FAJP\x02\x00"), Err(BoundaryError::Version(2)));
    assert_eq!(Boundaries::parse(b"FAJP\x01\x01"), Err(BoundaryError::Truncated));
    assert_eq!(Boundaries::parse(b"FAJP\x01\x01\x30\x00\x00"), Err(BoundaryError::Prefecture(0x30)));
    assert_eq!(Boundaries::parse(b"FAJP\x01\x00"), Ok(Boundaries { shapes: vec![] }));
}

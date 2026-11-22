use geodesy::{Bounds, Coordinate, Polygon, Projection};

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn distance_of_one_degree_of_latitude() {
    // One degree of a great circle on the mean sphere is 2πR/360.
    let expected = 2.0 * std::f64::consts::PI * Coordinate::EARTH_RADIUS_KM / 360.0;
    let a = Coordinate::new(35.0, 139.0);
    let b = Coordinate::new(36.0, 139.0);
    assert!(close(a.distance_km(&b), expected, 1e-9));
}

#[test]
fn distance_is_symmetric_and_zero_to_itself() {
    let tokyo = Coordinate::new(35.6812, 139.7671);
    let osaka = Coordinate::new(34.7025, 135.4959);
    assert!(close(tokyo.distance_km(&osaka), osaka.distance_km(&tokyo), 1e-9));
    assert_eq!(tokyo.distance_km(&tokyo), 0.0);
    // Tokyo station to Osaka station is about 403 km along a great circle.
    assert!(close(tokyo.distance_km(&osaka), 403.0, 5.0));
}

#[test]
fn antipodes_are_half_a_circumference_apart() {
    let a = Coordinate::new(10.0, 20.0);
    let b = Coordinate::new(-10.0, -160.0);
    // asin loses precision near the antipode; a metre is still far finer than needed.
    assert!(close(a.distance_km(&b), std::f64::consts::PI * Coordinate::EARTH_RADIUS_KM, 1e-3));
}

#[test]
fn bearings_of_the_cardinal_directions() {
    let origin = Coordinate::new(0.0, 0.0);
    assert!(close(origin.bearing_to(&Coordinate::new(1.0, 0.0)), 0.0, 1e-9));
    assert!(close(origin.bearing_to(&Coordinate::new(0.0, 1.0)), 90.0, 1e-9));
    assert!(close(origin.bearing_to(&Coordinate::new(-1.0, 0.0)), 180.0, 1e-9));
    assert!(close(origin.bearing_to(&Coordinate::new(0.0, -1.0)), 270.0, 1e-9));
}

#[test]
fn destination_inverts_distance_and_bearing() {
    let start = Coordinate::new(38.3, 142.4);
    for bearing in [0.0, 45.0, 135.0, 210.0, 300.0] {
        let end = start.destination(bearing, 250.0);
        assert!(close(start.distance_km(&end), 250.0, 1e-6));
        let difference = (start.bearing_to(&end) - bearing + 540.0) % 360.0 - 180.0;
        assert!(difference.abs() < 1e-6, "{bearing}: {difference}");
    }
}

#[test]
fn slant_distance_adds_the_depth() {
    let a = Coordinate::new(35.0, 139.0);
    assert!(close(a.slant_distance_km(&a, 30.0), 30.0, 1e-9));
    let b = a.destination(90.0, 40.0);
    assert!(close(a.slant_distance_km(&b, 30.0), 50.0, 1e-6));
}

#[test]
fn checked_rejects_out_of_range_values() {
    assert!(Coordinate::checked(91.0, 0.0).is_none());
    assert!(Coordinate::checked(0.0, 181.0).is_none());
    assert!(Coordinate::checked(f64::NAN, 0.0).is_none());
    // -200 is the "no hypocentre" value of the P2P Quake API.
    assert!(Coordinate::checked(-200.0, -200.0).is_none());
    assert!(Coordinate::checked(35.0, 139.0).is_some());
}

#[test]
fn polygon_containment_and_centroid() {
    let square = Polygon::new(vec![Coordinate::new(0.0, 0.0), Coordinate::new(0.0, 2.0), Coordinate::new(2.0, 2.0), Coordinate::new(2.0, 0.0)]);
    assert!(square.contains(&Coordinate::new(1.0, 1.0)));
    assert!(!square.contains(&Coordinate::new(3.0, 1.0)));
    assert!(!square.contains(&Coordinate::new(1.0, -0.5)));
    let centroid = square.centroid().unwrap();
    assert!(close(centroid.latitude, 1.0, 1e-12) && close(centroid.longitude, 1.0, 1e-12));
    assert!(close(square.signed_area().abs(), 4.0, 1e-12));
}

#[test]
fn concave_polygon_containment() {
    // A "U": the notch between the arms is outside.
    let u = Polygon::new(vec![
        Coordinate::new(0.0, 0.0),
        Coordinate::new(0.0, 3.0),
        Coordinate::new(3.0, 3.0),
        Coordinate::new(3.0, 2.0),
        Coordinate::new(1.0, 2.0),
        Coordinate::new(1.0, 1.0),
        Coordinate::new(3.0, 1.0),
        Coordinate::new(3.0, 0.0),
    ]);
    assert!(u.contains(&Coordinate::new(2.0, 0.5)));
    assert!(u.contains(&Coordinate::new(2.0, 2.5)));
    assert!(!u.contains(&Coordinate::new(2.0, 1.5)));
}

#[test]
fn bounds_hold_their_points() {
    let points = [Coordinate::new(30.0, 130.0), Coordinate::new(45.0, 145.0), Coordinate::new(35.0, 128.0)];
    let bounds = Bounds::of(&points).unwrap();
    assert_eq!(bounds, Bounds { south: 30.0, west: 128.0, north: 45.0, east: 145.0 });
    assert!(points.iter().all(|p| bounds.contains(p)));
    assert!(Bounds::of(&[]).is_none());
}

#[test]
fn web_mercator_known_points() {
    let p = Projection::WebMercator;
    let (x, y) = p.project(&Coordinate::new(0.0, 0.0));
    assert!(close(x, 0.5, 1e-12) && close(y, 0.5, 1e-12));
    let (x, y) = p.project(&Coordinate::new(Projection::MERCATOR_LIMIT, 180.0));
    assert!(close(x, 1.0, 1e-12) && close(y, 0.0, 1e-9));
    let back = p.unproject(0.7, 0.4);
    let (x, y) = p.project(&back);
    assert!(close(x, 0.7, 1e-12) && close(y, 0.4, 1e-12));
}

#[test]
fn equirectangular_round_trip() {
    let p = Projection::Equirectangular { reference_latitude: 36.0 };
    let point = Coordinate::new(35.5, 139.7);
    let (x, y) = p.project(&point);
    let back = p.unproject(x, y);
    assert!(close(back.latitude, point.latitude, 1e-12) && close(back.longitude, point.longitude, 1e-12));
    // North is up: a point further north has a smaller y.
    assert!(p.project(&Coordinate::new(40.0, 139.7)).1 < y);
}

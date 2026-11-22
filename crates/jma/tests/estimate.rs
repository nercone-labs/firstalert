//! The ground-motion estimate against the published relations it is made of.

use geodesy::Coordinate;
use jma::{Arrival, GroundMotion, Intensity, SiteAmplification};

#[test]
fn fujimoto_midorikawa_2005_at_known_velocities() {
    // I = 2.002 + 2.603 log PGV − 0.213 (log PGV)²
    assert!((GroundMotion::intensity_from_pgv(1.0) - 2.002).abs() < 1e-12);
    assert!((GroundMotion::intensity_from_pgv(10.0) - (2.002 + 2.603 - 0.213)).abs() < 1e-12);
    assert!((GroundMotion::intensity_from_pgv(100.0) - (2.002 + 2.0 * 2.603 - 4.0 * 0.213)).abs() < 1e-12);
}

#[test]
fn si_midorikawa_1999_by_hand() {
    // A site 40 km from the epicentre of an M6.0 earthquake 30 km deep.
    let epicentre = Coordinate::new(36.0, 140.0);
    let site = epicentre.destination(90.0, 40.0);
    let motion = GroundMotion::estimate(&site, &epicentre, 30.0, 6.0, SiteAmplification::FIRM).unwrap();
    let hypocentral = (40.0f64 * 40.0 + 30.0 * 30.0).sqrt();
    assert!((motion.hypocentral_distance_km - hypocentral).abs() < 1e-6);
    let fault_length = 10f64.powf(0.6 * 6.0 - 2.9);
    let x = hypocentral - fault_length / 2.0;
    let log_pgv600 = 0.58 * 6.0 + 0.0038 * 30.0 - 1.29 - (x + 0.0028 * 10f64.powf(3.0)).log10() - 0.002 * x;
    let pgv = 10f64.powf(log_pgv600) * 1.41;
    assert!((motion.peak_ground_velocity - pgv).abs() < 1e-9);
    assert!((motion.measured_intensity - GroundMotion::intensity_from_pgv(pgv)).abs() < 1e-12);
}

#[test]
fn shaking_weakens_with_distance_and_grows_with_magnitude() {
    let epicentre = Coordinate::new(35.0, 139.0);
    let near = epicentre.destination(0.0, 20.0);
    let far = epicentre.destination(0.0, 200.0);
    let at = |site: &Coordinate, m: f64| GroundMotion::estimate(site, &epicentre, 10.0, m, SiteAmplification::FIRM).unwrap().measured_intensity;
    assert!(at(&near, 6.0) > at(&far, 6.0));
    assert!(at(&near, 7.0) > at(&near, 6.0));
    // Softer ground shakes more.
    let soft = GroundMotion::estimate(&near, &epicentre, 10.0, 6.0, SiteAmplification(2.0)).unwrap().measured_intensity;
    assert!(soft > at(&near, 6.0));
}

#[test]
fn plausible_classes_for_familiar_cases() {
    // An M7.3 shallow crustal earthquake gives 6弱 or more close to the fault, and a small M3
    // earthquake 100 km away is hardly felt.
    let epicentre = Coordinate::new(32.75, 130.76);
    let close = epicentre.destination(45.0, 5.0);
    let strong = GroundMotion::estimate(&close, &epicentre, 12.0, 7.3, SiteAmplification::FIRM).unwrap();
    assert!(Intensity::from_measured(strong.measured_intensity).unwrap() >= Intensity::SixLower, "{strong:?}");
    let distant = epicentre.destination(45.0, 100.0);
    let weak = GroundMotion::estimate(&distant, &epicentre, 10.0, 3.0, SiteAmplification::FIRM).unwrap();
    assert!(Intensity::from_measured(weak.measured_intensity).unwrap() <= Intensity::One, "{weak:?}");
}

#[test]
fn magnitude_is_capped_at_the_limit_of_the_relation() {
    let epicentre = Coordinate::new(38.0, 143.0);
    let site = epicentre.destination(270.0, 300.0);
    let a = GroundMotion::estimate(&site, &epicentre, 20.0, 8.3, SiteAmplification::FIRM).unwrap();
    let b = GroundMotion::estimate(&site, &epicentre, 20.0, 9.0, SiteAmplification::FIRM).unwrap();
    // The cap applies to the attenuation; a longer fault still brings the site closer.
    assert!(b.measured_intensity >= a.measured_intensity);
}

#[test]
fn nonsense_input_gives_no_estimate() {
    let p = Coordinate::new(35.0, 139.0);
    assert!(GroundMotion::estimate(&p, &p, -1.0, 5.0, SiteAmplification::FIRM).is_none());
    assert!(GroundMotion::estimate(&p, &p, 10.0, -1.0, SiteAmplification::FIRM).is_none());
    assert!(GroundMotion::estimate(&p, &p, 10.0, f64::NAN, SiteAmplification::FIRM).is_none());
    assert!(GroundMotion::estimate(&p, &p, 10.0, 5.0, SiteAmplification(0.0)).is_none());
}

#[test]
fn arrivals_and_radii() {
    let arrival = Arrival::at(90.0);
    assert!((arrival.p_seconds - 90.0 / 7.8).abs() < 1e-12);
    assert!((arrival.s_seconds - 20.0).abs() < 1e-12);
    assert!(arrival.p_seconds < arrival.s_seconds);
    // Before a wave reaches the surface it has no circle; then the circle grows.
    assert_eq!(Arrival::radii_km(1.0, 30.0), (None, None));
    let (p, s) = Arrival::radii_km(10.0, 30.0);
    assert!((p.unwrap() - (78.0f64 * 78.0 - 900.0).sqrt()).abs() < 1e-9);
    assert!((s.unwrap() - (45.0f64 * 45.0 - 900.0).sqrt()).abs() < 1e-9);
}

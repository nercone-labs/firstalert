//! The seismic intensity scale, as JMA defines it ("計測震度の算出方法", and the table of the
//! classes: 震度0 below 0.5, …, 震度7 from 6.5), and its written forms.

use jma::{Intensity, IntensityRange};

#[test]
fn classes_from_measured_intensity_follow_the_thresholds() {
    let cases = [
        (-1.0, Intensity::Zero),
        (0.0, Intensity::Zero),
        (0.4, Intensity::Zero),
        (0.5, Intensity::One),
        (1.4, Intensity::One),
        (1.5, Intensity::Two),
        (2.5, Intensity::Three),
        (3.5, Intensity::Four),
        (4.4, Intensity::Four),
        (4.5, Intensity::FiveLower),
        (4.9, Intensity::FiveLower),
        (5.0, Intensity::FiveUpper),
        (5.4, Intensity::FiveUpper),
        (5.5, Intensity::SixLower),
        (6.0, Intensity::SixUpper),
        (6.4, Intensity::SixUpper),
        (6.5, Intensity::Seven),
        (7.2, Intensity::Seven),
    ];
    for (measured, class) in cases {
        assert_eq!(Intensity::from_measured(measured), Some(class), "measured {measured}");
    }
}

#[test]
fn measured_intensity_is_rounded_to_a_tenth_first() {
    // 4.46 is 4.5 to one decimal place, so 5弱; 4.44 is 4.4, so 4.
    assert_eq!(Intensity::from_measured(4.46), Some(Intensity::FiveLower));
    assert_eq!(Intensity::from_measured(4.44), Some(Intensity::Four));
    assert_eq!(Intensity::from_measured(0.45), Some(Intensity::One));
    assert_eq!(Intensity::from_measured(f64::NAN), None);
}

#[test]
fn every_class_starts_at_its_lower_bound() {
    for class in Intensity::ALL.iter().skip(1) {
        assert_eq!(Intensity::from_measured(class.lower_bound()), Some(*class));
        assert_eq!(Intensity::from_measured(class.lower_bound() - 0.1), Some(Intensity::ALL[*class as usize - 1]));
    }
}

#[test]
fn written_forms() {
    let forms = [
        ("0", Intensity::Zero),
        ("1", Intensity::One),
        ("4", Intensity::Four),
        ("5-", Intensity::FiveLower),
        ("5+", Intensity::FiveUpper),
        ("6-", Intensity::SixLower),
        ("6+", Intensity::SixUpper),
        ("7", Intensity::Seven),
        ("5弱", Intensity::FiveLower),
        ("5強", Intensity::FiveUpper),
        ("６弱", Intensity::SixLower),
        ("６強", Intensity::SixUpper),
        ("震度７", Intensity::Seven),
        ("震度3", Intensity::Three),
        (" 2 ", Intensity::Two),
    ];
    for (text, class) in forms {
        assert_eq!(Intensity::parse(text), Some(class), "{text}");
    }
    for text in ["", "不明", "8", "5", "6", "-1", "５弱以上"] {
        assert_eq!(Intensity::parse(text), None, "{text}");
    }
}

#[test]
fn codes_round_trip() {
    for class in Intensity::ALL {
        assert_eq!(Intensity::parse(class.code()), Some(class));
        let json = format!("\"{}\"", class.code());
        assert_eq!(serde_json_like(class), json);
    }
}

fn serde_json_like(class: Intensity) -> String {
    format!("\"{class}\"")
}

#[test]
fn p2p_quake_scale() {
    let scales = [
        (0, Intensity::Zero),
        (10, Intensity::One),
        (20, Intensity::Two),
        (30, Intensity::Three),
        (40, Intensity::Four),
        (45, Intensity::FiveLower),
        (46, Intensity::FiveLower),
        (50, Intensity::FiveUpper),
        (55, Intensity::SixLower),
        (60, Intensity::SixUpper),
        (70, Intensity::Seven),
    ];
    for (scale, class) in scales {
        assert_eq!(Intensity::from_scale(scale), Some(class));
    }
    for scale in [-1, 5, 15, 99, 80] {
        assert_eq!(Intensity::from_scale(scale), None);
    }
}

#[test]
fn classes_are_ordered() {
    assert!(Intensity::FiveLower > Intensity::Four);
    assert!(Intensity::FiveUpper > Intensity::FiveLower);
    assert!(Intensity::Seven > Intensity::SixUpper);
    assert_eq!(Intensity::ALL.iter().max(), Some(&Intensity::Seven));
}

#[test]
fn ranges() {
    let open = IntensityRange::at_least(Intensity::FiveLower);
    assert_eq!(open.highest(), None);
    assert_eq!(open.strongest_known(), Intensity::FiveLower);
    let closed = IntensityRange { lower: Intensity::Four, upper: Some(Intensity::FiveUpper) };
    assert_eq!(closed.highest(), Some(Intensity::FiveUpper));
    assert_eq!(closed.strongest_known(), Intensity::FiveUpper);
    assert_eq!(IntensityRange::exactly(Intensity::Three).strongest_known(), Intensity::Three);
}

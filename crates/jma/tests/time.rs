use jma::Time;

// 2026-11-22 17:43:49 JST is 2026-11-22 08:43:49 UTC.
const SAMPLE_UTC_MILLIS: i64 = 1_795_337_029_000;

#[test]
fn jst_times_of_the_relays() {
    assert_eq!(Time::parse_jst("2026/11/22 17:43:49"), Some(Time(SAMPLE_UTC_MILLIS)));
    assert_eq!(Time::parse_jst("2026/11/22 17:43"), Some(Time(SAMPLE_UTC_MILLIS - 49_000)));
    assert_eq!(Time::parse_jst("2026/11/22 17:43:49.258"), Some(Time(SAMPLE_UTC_MILLIS + 258)));
    assert_eq!(Time::parse_jst("2026/11/22 17:43:49.25"), Some(Time(SAMPLE_UTC_MILLIS + 250)));
    assert_eq!(Time::parse_jst("2026-11-22 17:43:49"), Some(Time(SAMPLE_UTC_MILLIS)));
}

#[test]
fn the_epoch_and_the_day_boundary() {
    assert_eq!(Time::parse_jst("1970/01/01 09:00:00"), Some(Time(0)));
    // Midnight in Japan is 15:00 UTC the day before.
    assert_eq!(Time::parse_jst("2026/01/01 00:00:00"), Time::parse_rfc3339("2025-12-31T15:00:00Z"));
}

#[test]
fn impossible_dates_are_rejected() {
    for text in [
        "2026/02/29 00:00",
        "2024/13/01 00:00",
        "2026/04/31 00:00",
        "2026/11/22 24:00",
        "2026/11/22 12:60",
        "2026/11/22",
        "",
        "2026/11/22 ab:cd",
        "2026/11/22 17:43:49.x",
    ] {
        assert_eq!(Time::parse_jst(text), None, "{text}");
    }
    // 2024 is a leap year; 1900 is not; 2000 is.
    assert!(Time::parse_jst("2024/02/29 00:00").is_some());
    assert!(Time::parse_jst("1900/02/29 00:00").is_none());
    assert!(Time::parse_jst("2000/02/29 00:00").is_some());
}

#[test]
fn rfc3339() {
    assert_eq!(Time::parse_rfc3339("2026-11-22T08:43:49.000Z"), Some(Time(SAMPLE_UTC_MILLIS)));
    assert_eq!(Time::parse_rfc3339("2026-11-22T08:43:49Z"), Some(Time(SAMPLE_UTC_MILLIS)));
    assert_eq!(Time::parse_rfc3339("2026-11-22T17:43:49+09:00"), Some(Time(SAMPLE_UTC_MILLIS)));
    assert_eq!(Time::parse_rfc3339("2026-11-22T03:43:49-05:00"), Some(Time(SAMPLE_UTC_MILLIS)));
    assert_eq!(Time::parse_rfc3339("2026-11-22 08:43:49"), None);
}

#[test]
fn formatting_in_jst() {
    let time = Time(SAMPLE_UTC_MILLIS);
    assert_eq!(time.format_jst(), "2026/11/22 17:43:49");
    assert_eq!(time.jst_fields(), (2026, 11, 22, 17, 43, 49));
    assert_eq!(Time::parse_jst(&time.format_jst()), Some(time));
    // Before the epoch.
    let early = Time::parse_jst("1960/03/01 08:30:00").unwrap();
    assert_eq!(early.format_jst(), "1960/03/01 08:30:00");
}

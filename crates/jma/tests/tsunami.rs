use jma::TsunamiGrade;

#[test]
fn grades_are_ordered_by_severity() {
    assert!(TsunamiGrade::MajorWarning > TsunamiGrade::Warning);
    assert!(TsunamiGrade::Warning > TsunamiGrade::Advisory);
    assert!(TsunamiGrade::Advisory > TsunamiGrade::Forecast);
    assert!(TsunamiGrade::Forecast > TsunamiGrade::Unknown);
}

#[test]
fn warnings_and_advisories_call_for_leaving_the_coast() {
    assert!(TsunamiGrade::MajorWarning.calls_for_action());
    assert!(TsunamiGrade::Warning.calls_for_action());
    assert!(TsunamiGrade::Advisory.calls_for_action());
    assert!(!TsunamiGrade::Forecast.calls_for_action());
    assert!(!TsunamiGrade::Unknown.calls_for_action());
}

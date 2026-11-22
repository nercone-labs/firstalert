use lenient::Lenient;
use serde_json::json;

#[test]
fn values_of_the_expected_type() {
    let v = json!({"a": {"b": "text", "n": 3.5, "i": 7, "t": true, "list": [1, 2]}});
    assert_eq!(v.string(&["a", "b"]).as_deref(), Some("text"));
    assert_eq!(v.number(&["a", "n"]), Some(3.5));
    assert_eq!(v.integer(&["a", "i"]), Some(7));
    assert_eq!(v.boolean(&["a", "t"]), Some(true));
    assert_eq!(v.elements(&["a", "list"]).len(), 2);
}

#[test]
fn values_of_another_type_are_converted_when_unambiguous() {
    let v = json!({"n": "3.7", "i": "10", "f": 4.0, "s": 12, "t": "false", "one": 1});
    assert_eq!(v.number(&["n"]), Some(3.7));
    assert_eq!(v.integer(&["i"]), Some(10));
    assert_eq!(v.integer(&["f"]), Some(4));
    assert_eq!(v.string(&["s"]).as_deref(), Some("12"));
    assert_eq!(v.boolean(&["t"]), Some(false));
    assert_eq!(v.boolean(&["one"]), Some(true));
}

#[test]
fn missing_null_and_wrong_values_are_absent() {
    let v = json!({"null": null, "obj": {}, "n": "abc", "f": 4.5, "s": "  ", "arr": "x"});
    assert_eq!(v.string(&["missing"]), None);
    assert_eq!(v.string(&["null"]), None);
    assert_eq!(v.string(&["obj"]), None);
    assert_eq!(v.number(&["n"]), None);
    assert_eq!(v.integer(&["f"]), None);
    assert_eq!(v.text(&["s"]), None);
    assert_eq!(v.string(&["s"]).as_deref(), Some("  "));
    assert!(v.elements(&["arr"]).is_empty());
    assert_eq!(v.string(&["obj", "deeper", "still"]), None);
    assert_eq!(v.boolean(&["n"]), None);
}

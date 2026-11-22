//! The C interface, called as C would call it.

use firstalert_ffi::{firstalert_buffer_free, firstalert_core_free, firstalert_core_handle, firstalert_core_new};

fn call(core: *mut firstalert_ffi::FirstAlertCore, input: &str) -> String {
    let mut length = 0usize;
    let buffer = unsafe { firstalert_core_handle(core, input.as_ptr(), input.len(), &mut length) };
    assert!(!buffer.is_null());
    let text = unsafe { std::str::from_utf8(std::slice::from_raw_parts(buffer, length)).unwrap().to_owned() };
    unsafe { firstalert_buffer_free(buffer, length) };
    text
}

#[test]
fn a_core_answers_in_json() {
    let core = firstalert_core_new();
    let started = call(core, r#"{"type":"start","now":0,"data_dir":null,"locale":"ja"}"#);
    assert!(started.contains(r#""type":"connect""#));
    assert!(started.contains(r#""snapshot""#));
    let error = call(core, "{");
    assert!(error.contains(r#""error":"invalid input"#));
    let not_utf8 = {
        let bytes = [0xffu8, 0xfe];
        let mut length = 0usize;
        let buffer = unsafe { firstalert_core_handle(core, bytes.as_ptr(), bytes.len(), &mut length) };
        let text = unsafe { std::str::from_utf8(std::slice::from_raw_parts(buffer, length)).unwrap().to_owned() };
        unsafe { firstalert_buffer_free(buffer, length) };
        text
    };
    assert!(not_utf8.contains("not UTF-8"));
    unsafe { firstalert_core_free(core) };
}

#[test]
fn null_pointers_are_refused() {
    let mut length = 0usize;
    assert!(unsafe { firstalert_core_handle(std::ptr::null_mut(), std::ptr::null(), 0, &mut length) }.is_null());
    let core = firstalert_core_new();
    assert!(unsafe { firstalert_core_handle(core, std::ptr::null(), 0, std::ptr::null_mut()) }.is_null());
    // An empty input is invalid JSON, answered with an error.
    let buffer = unsafe { firstalert_core_handle(core, std::ptr::null(), 0, &mut length) };
    assert!(!buffer.is_null());
    unsafe { firstalert_buffer_free(buffer, length) };
    unsafe { firstalert_core_free(core) };
    unsafe { firstalert_core_free(std::ptr::null_mut()) };
}

//! The Java Native Interface of the core, for the class `dev.nercone.firstalert.core.NativeCore`:
//!
//! ```kotlin
//! object NativeCore {
//!     @JvmStatic external fun create(): Long
//!     @JvmStatic external fun handle(core: Long, input: ByteArray): ByteArray?
//!     @JvmStatic external fun destroy(core: Long)
//! }
//! ```
//!
//! Byte arrays carry UTF-8, not Java strings, whose modified UTF-8 would need converting.

use crate::FirstAlertCore;
use jni_sys::{JNIEnv, jbyte, jbyteArray, jclass, jlong, jsize};

/// # Safety
/// Called by the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_dev_nercone_firstalert_core_NativeCore_create(_env: *mut JNIEnv, _class: jclass) -> jlong {
    crate::firstalert_core_new() as jlong
}

/// # Safety
/// Called by the JVM with a handle from `create` not yet destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_dev_nercone_firstalert_core_NativeCore_destroy(_env: *mut JNIEnv, _class: jclass, core: jlong) {
    // SAFETY: the handle came from `create`.
    unsafe { crate::firstalert_core_free(core as *mut FirstAlertCore) };
}

/// Returns the response, or null when the input could not be read or the array could not be
/// made (an exception is then pending in the JVM).
///
/// # Safety
/// Called by the JVM with a handle from `create`, from one thread at a time.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_dev_nercone_firstalert_core_NativeCore_handle(
    env: *mut JNIEnv,
    _class: jclass,
    core: jlong,
    input: jbyteArray,
) -> jbyteArray {
    if core == 0 || input.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: `env` is the JNI environment of this thread, given by the JVM; the functions of
    // versions 1.1 and 1.2 are present in every JVM Android and the desktops run.
    unsafe {
        let functions = (**env).v1_1;
        let exception_check = (**env).v1_2.ExceptionCheck;
        let length = (functions.GetArrayLength)(env, input);
        if length < 0 {
            return std::ptr::null_mut();
        }
        let mut bytes = vec![0u8; length as usize];
        (functions.GetByteArrayRegion)(env, input, 0, length, bytes.as_mut_ptr() as *mut jbyte);
        if exception_check(env) {
            return std::ptr::null_mut();
        }
        let core = &mut *(core as *mut FirstAlertCore);
        let response = core.handle(&bytes);
        let Ok(size) = jsize::try_from(response.len()) else { return std::ptr::null_mut() };
        let array = (functions.NewByteArray)(env, size);
        if array.is_null() {
            return std::ptr::null_mut();
        }
        (functions.SetByteArrayRegion)(env, array, 0, size, response.as_ptr() as *const jbyte);
        array
    }
}

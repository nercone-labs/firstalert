package dev.nercone.firstalert.core

/**
 * The Rust core (`crates/firstalert-ffi`, the library `libfirstalert_ffi.so`), through the Java
 * Native Interface. Inputs and responses are UTF-8 JSON (`firstalert::protocol`). A core is not
 * thread-safe: `Core` serialises the calls.
 */
object NativeCore {
    init {
        System.loadLibrary("firstalert_ffi")
    }

    @JvmStatic external fun create(): Long

    /** The response to `input`; `null` only when the JVM could not pass the arrays. */
    @JvmStatic external fun handle(core: Long, input: ByteArray): ByteArray?

    @JvmStatic external fun destroy(core: Long)
}

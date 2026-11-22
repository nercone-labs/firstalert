# The Rust core calls no Java, but the JVM binds its native methods by the names of the class
# and the methods (`Java_dev_nercone_firstalert_core_NativeCore_*`).
-keep class dev.nercone.firstalert.core.NativeCore { *; }

# kotlinx.serialization: keep the generated serializers of the core models.
-keepclassmembers @kotlinx.serialization.Serializable class dev.nercone.firstalert.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class dev.nercone.firstalert.**$$serializer { *; }

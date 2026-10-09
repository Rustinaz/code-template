# Nothing to keep: the app has no Java/Kotlin reflection surface, and all of the
# UI, state and networking is compiled Rust inside the .so files.
#
# R8 cannot see through the JNI entry points declared in the manifest, so the one
# thing worth keeping is a rule that stops it from warning about the activity
# the framework instantiates for us.
-keep class android.app.NativeActivity { *; }

# egui/wgpu pull in these through C; R8 cannot see the native references.
-dontwarn org.eclipse.jdt.annotation.**
-dontwarn java.lang.management.**
-dontwarn jdk.internal.**

// Top-level Gradle settings for the Android packaging of `example-app`.
//
// This project contains no Kotlin and no XML layouts: the manifest points at
// the framework's own `android.app.NativeActivity`, and everything the user sees
// is drawn by Rust (egui) into the native surface.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "rust-crossplatform-template"

include(":app")

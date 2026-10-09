// Root build file. The Android plugin is declared here and applied in `:app`.
plugins {
    id("com.android.application") version "8.7.3" apply false
}

// Convenience: `./gradlew listAbis` shows what will be built.
tasks.register("listAbis") {
    group = "help"
    description = "Prints the ABIs the native library is built for."
    val abis = listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
    doLast {
        abis.forEach { println(it) }
    }
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}

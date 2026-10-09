// Packaging for `example-app` on Android.
//
// There is no Kotlin or Java source in this module — only the manifest, a
// theme and a launcher icon. Everything else is the Rust `.so`.
//
// How the APK is produced:
//   1. `cargoBuild` runs `cargo build --target <triple>` once per ABI with the
//      NDK's clang as linker. The result is `libexample_app.so`.
//   2. The shared objects are staged into `build/rustJniLibs/<abi>/`.
//   3. AGP picks them up as `jniLibs` and packages them into the APK/AAB.
//
// Point Cargo at the NDK with either `ANDROID_NDK_HOME` or
// `-PandroidNdk=/path/to/ndk`. `scripts/build-android.sh` does that for you.

plugins {
    id("com.android.application")
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Cargo package to compile. This is the *package* name, so it matches
 * `name =` in `apps/example/Cargo.toml`.
 */
val rustPackage = "example-app"

/**
 * Name of the produced shared object, `lib<rustLibName>.so`.
 *
 * Must match `[lib] name` in `apps/example/Cargo.toml` **and**
 * `android.app.lib_name` in `src/main/AndroidManifest.xml`.
 */
val rustLibName = "example_app"

/**
 * Path to the Cargo workspace root.
 *
 * This file lives in `android-app/app/`, so the workspace is two levels up --
 * `..` would be `android-app/`, which has no `Cargo.toml`.
 */
val workspaceRoot = file("../..").canonicalFile

/** Where the cross-compiled shared objects are staged for AGP. */
val rustJniLibs = layout.buildDirectory.dir("rustJniLibs")

/** ABIs to build. Override with `-PandroidAbi=arm64-v8a,x86_64`. */
val targetAbis: List<String> =
    (project.findProperty("androidAbi") as String? ?: "arm64-v8a,armeabi-v7a,x86_64,x86")
        .split(",")
        .map(String::trim)
        .filter(String::isNotEmpty)

/** Android ABI -> Rust target triple. */
fun abiToTriple(abi: String): String = when (abi) {
    "arm64-v8a" -> "aarch64-linux-android"
    "armeabi-v7a" -> "armv7-linux-androideabi"
    "x86_64" -> "x86_64-linux-android"
    "x86" -> "i686-linux-android"
    else -> throw GradleException(
        "Unknown ABI '$abi'. Use one of arm64-v8a, armeabi-v7a, x86_64, x86.",
    )
}

/** Android ABI -> NDK clang architecture prefix. */
fun abiToArchPrefix(abi: String): String = when (abi) {
    "arm64-v8a" -> "aarch64"
    "armeabi-v7a" -> "armv7a"
    "x86_64" -> "x86_64"
    "x86" -> "i686"
    else -> throw GradleException("Unknown ABI '$abi'")
}

/** Triple suffix, which is `android` except on 32-bit ARM. */
fun abiToTripleSuffix(abi: String): String =
    if (abi == "armeabi-v7a") "androideabi" else "android"

/** Minimum API level; the NDK toolchain is also named for this level. */
val minApi = 21

// ---------------------------------------------------------------------------
// NDK discovery
// ---------------------------------------------------------------------------

/**
 * Sort key for an NDK directory name.
 *
 * NDK directories are named `27.1.12297006` or `android-ndk-r27d`, so a plain
 * string sort would put `android-ndk-r27d` after `27.1.12297006` and pick the
 * wrong one. Padding every digit run to a fixed width makes the comparison
 * numeric again, so `27.1.12297006` sorts above `27.0.12077973` and the newest
 * NDK wins.
 */
fun ndkVersionKey(name: String): String =
    Regex("\\d+").findAll(name).joinToString(".") { it.value.padStart(10, '0') }

/** Locates the NDK, preferring the explicit `-PandroidNdk` value. */
fun findNdkHome(): File? {
    val explicit = project.findProperty("androidNdk") as String?
    if (explicit != null) {
        val dir = file(explicit)
        if (dir.isDirectory) return dir
        throw GradleException("androidNdk=$explicit is not a directory")
    }

    val bases = listOfNotNull(
        System.getenv("ANDROID_NDK_HOME"),
        System.getenv("ANDROID_NDK_ROOT"),
        System.getenv("ANDROID_HOME")?.let { "$it/ndk" },
        System.getenv("ANDROID_SDK_ROOT")?.let { "$it/ndk" },
    )

    for (base in bases) {
        val dir = file(base)
        if (!dir.isDirectory) continue
        // $ANDROID_NDK_HOME may already point at a versioned directory.
        if (File(dir, "source.properties").isFile) return dir
        // Otherwise take the highest installed version, sorted semantically so
        // that r27.1.12297006 sorts after r27.0.12077973.
        dir.listFiles()
            ?.filter { File(it, "source.properties").isFile }
            ?.maxByOrNull { ndkVersionKey(it.name) }
            ?.let { return it }
    }
    return null
}

/** The `toolchains/llvm/prebuilt/<host>/bin` directory of an NDK. */
fun ndkBinDir(ndkHome: File): File {
    val prebuilt = File(ndkHome, "toolchains/llvm/prebuilt")
    val host = prebuilt.listFiles()?.firstOrNull { it.isDirectory } ?: prebuilt
    return File(host, "bin").also {
        if (!it.isDirectory) {
            throw GradleException("NDK has no llvm/prebuilt toolchain at ${host.absolutePath}")
        }
    }
}

/**
 * The clang driver for `abi`.
 *
 * NDK r23 and newer removed the bare `<arch>-linux-android-clang` symlinks, so
 * the API level has to be part of the name.
 */
fun clangFor(binDir: File, abi: String, apiLevel: Int): File {
    val prefix = "${abiToArchPrefix(abi)}-linux-${abiToTripleSuffix(abi)}"
    return File(binDir, "$prefix$apiLevel-clang").takeIf { it.isFile }
        ?: throw GradleException(
            "No clang for $abi in ${binDir.absolutePath}. " +
                "NDK r23+ requires the API level in the toolchain name; " +
                "check that NDK $apiLevel or newer is installed.",
        )
}

/** Cargo/cc-rs spell target triples in `SCREAMING_SNAKE` for linker vars... */
fun cargoTargetEnv(triple: String): String =
    "CARGO_TARGET_" + triple.replace('-', '_').uppercase()

/** ...and in `lower_snake` for the C-toolchain vars. */
fun ccEnv(prefix: String, triple: String): String =
    "${prefix}_" + triple.replace('-', '_').lowercase()

// ---------------------------------------------------------------------------
// Cargo cross-compilation
// ---------------------------------------------------------------------------

val cargoBuild = tasks.register("cargoBuild") {
    group = "rust"
    description = "Cross-compiles $rustPackage for ${targetAbis.joinToString()}"

    doLast {
        val ndkHome = findNdkHome() ?: throw GradleException(
            """
            Android NDK not found. Install it from Android Studio
            (SDK Manager -> SDK Tools -> NDK), then either:
              * set ANDROID_NDK_HOME=/path/to/ndk/<version>, or
              * pass -PandroidNdk=/path/to/ndk/<version>
            scripts/build-android.sh sets the variable for you.
            """.trimIndent(),
        )
        val binDir = ndkBinDir(ndkHome)
        logger.lifecycle("Using NDK at ${ndkHome.absolutePath}")

        val release = project.hasProperty("release")
        val profileArgs = if (release) listOf("--release") else emptyList()
        val profileName = if (release) "release" else "debug"

        for (abi in targetAbis) {
            val triple = abiToTriple(abi)
            val clang = clangFor(binDir, abi, minApi)
            val clangxx = File(binDir, "${clang.name.replace("clang", "clang++")}")
            val ar = File(binDir, "llvm-ar")
            val ranlib = File(binDir, "llvm-ranlib")

            val command = listOf(
                "cargo", "build",
                "--package", rustPackage,
                "--target", triple,
            ) + profileArgs

            logger.lifecycle("cargo ${command.joinToString(" ")}")

            // `ProcessBuilder` rather than Gradle's exec APIs: the result type
            // of `providers.exec` has changed shape across Gradle 8.x releases,
            // and this needs to keep working on whatever wrapper ships.
            //
            // stdout and stderr are captured and logged rather than inherited,
            // because the Gradle daemon's own file descriptors are not the
            // terminal the developer is watching.
            val process = ProcessBuilder(command)
                .directory(workspaceRoot)
                .apply {
                    // cc-rs finds the C compiler through `CC_*`, so the NDK has
                    // to be named there too, not just as the Rust linker.
                    environment()[cargoTargetEnv(triple) + "_LINKER"] = clang.absolutePath
                    environment()[ccEnv("CC", triple)] = clang.absolutePath
                    environment()[ccEnv("CXX", triple)] = clangxx.absolutePath
                    environment()[ccEnv("AR", triple)] = ar.absolutePath
                    environment()[ccEnv("RANLIB", triple)] = ranlib.absolutePath
                    // Keeps the clang driver in PATH for build scripts that
                    // shell out to it by bare name.
                    environment()["PATH"] =
                        binDir.absolutePath + File.pathSeparator + System.getenv("PATH")
                }
                .start()

            val output = process.inputStream.bufferedReader().readText()
            val errors = process.errorStream.bufferedReader().readText()
            val exitValue = process.waitFor()
            logger.lifecycle(output.trimEnd())
            if (errors.isNotBlank()) logger.warn(errors.trimEnd())
            if (exitValue != 0) {
                throw GradleException("cargo build failed for $abi (exit $exitValue)")
            }

            // Cargo writes a cross-compiled artifact to target/<triple>/<profile>/
            // -- the triple comes *before* the profile. A host build has no
            // triple at all, which is what makes this order easy to get wrong.
            val soFile = workspaceRoot
                .resolve("target")
                .resolve(triple)
                .resolve(profileName)
                .resolve("lib$rustLibName.so")
            if (!soFile.isFile) {
                throw GradleException(
                    "Cargo reported success but ${soFile.absolutePath} does not exist. " +
                        "Check that [lib] crate-type in apps/example/Cargo.toml is cdylib.",
                )
            }

            val stageDir = rustJniLibs.get().dir(abi).asFile
            stageDir.deleteRecursively()
            stageDir.mkdirs()
            soFile.copyTo(File(stageDir, "lib$rustLibName.so"), overwrite = true)
            logger.lifecycle("Staged lib$rustLibName.so for $abi (${soFile.length() / 1024} KiB)")
        }
    }
}

// ---------------------------------------------------------------------------
// Android application module
// ---------------------------------------------------------------------------

android {
    namespace = "dev.rustcrossplatform"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.rustcrossplatform"
        minSdk = minApi
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += targetAbis
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir(rustJniLibs)
        }
    }

    buildTypes {
        debug {
            isMinifyEnabled = false
            isJniDebuggable = true
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            // Replace with a real signingConfig before shipping anything.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    // Extract the .so instead of loading it straight from the APK: it keeps
    // `adb install` fast and avoids issues with compressed native libraries.
    packaging {
        jniLibs {
            useLegacyPackaging = true
            // Keep AGP's hands off the `.so` files.
            //
            // By default AGP runs its own ELF stripper over every packaged
            // native library. On a cross-compiled Rust cdylib it does not
            // recognise the symbol table, and for `x86_64-linux-android` it
            // produced a 24-byte stub in place of the real 166 MB library --
            // an APK that installs and then crashes on start-up.
            //
            // Stripping is Cargo's job: `[profile.release]` sets
            // `strip = "symbols"`, which runs llvm-strip from the NDK that
            // produced the object in the first place. Debug builds keep their
            // symbols so the NDK debugger can symbolicate native crashes.
            keepDebugSymbols += setOf("**/*.so")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// The .so must exist before AGP merges jniLibs, so every Android task that can
// package them depends on the Cargo build.
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("NativeLibs") }
    .configureEach { dependsOn(cargoBuild) }
tasks.named("preBuild") { dependsOn(cargoBuild) }

dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}

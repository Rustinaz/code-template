# Rustinaz

**A Rust-first cross-platform application template.** One codebase that builds a real
application for **Linux, Windows, macOS, Android, iOS and the web** — with a shared UI written
once in egui, a thin per-platform layer for what genuinely differs, and a bundled backend in
the same workspace.

[![CI](https://github.com/Rustinaz/code-template/actions/workflows/ci.yml/badge.svg)](https://github.com/Rustinaz/code-template/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macos%20%7C%20android%20%7C%20ios%20%7C%20web-blue)](#building-each-platform)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-green)](#licence)

> 🇮🇷 **نسخهٔ فارسیٔ این راهنما:** [README.fa.md](README.fa.md)

---

## What Rustinaz is

There is no Java or Kotlin **source code**, no XML layouts, no Swift, and no HTML/CSS user
interface. The Android app is one Rust `.so`; the web app is the same Rust compiled to WASM.
The Android project's Gradle build scripts are written in Kotlin DSL and it carries a few XML
resource files (manifest, theme, launcher icon), but neither is application code and neither
draws a pixel — the APK contains zero `.class` files.

It also ships a small pure-Rust backend in the same workspace (`apps/server`), whose request
and response types live once in `shared` and are imported by both ends, so the server and the
client are written and run from this one directory.

Most "cross-platform" starter kits do one of three things: they stop at desktop, they hide a
WebView behind a native shell, or they make you write the UI once per platform. Rustinaz takes
the opposite bet — **write the UI in Rust, ship the same widget tree to every target, and keep
only the genuinely OS-specific parts (window, theme, device info, accessibility, JNI) in
per-platform crates.** One language, one build system, one UI.

## Highlights

- **Six platforms, one UI.** egui everywhere — desktop, Android, iOS and the browser.
- **Android with no Java or Kotlin.** The manifest loads `android.app.NativeActivity` and the
  Rust `cdylib`; the APK has zero `.class` files.
- **A shared/platform split you can see.** `crates/ui` is the shared component library;
  `crates/platform/*` owns only what the OS must be told.
- **A backend in the same repository.** `shared::api` is the single wire contract, so the
  client and the server cannot drift apart.
- **Runs before you configure anything.** `cargo run` on desktop; `scripts/dev.sh` starts the
  backend and the client together.
- **Honest engineering.** Every gate is a command you can run, and this README states exactly
  what was verified and what was not.

## Table of contents

- [What Rustinaz is](#what-rustinaz-is)
- [Highlights](#highlights)
- [Get the code](#get-the-code)
- [Prerequisites](#prerequisites)
- [Step 1: run it on Linux](#step-1-run-it-on-linux)
- [Step 2: the desktop + Android loop](#step-2-the-desktop--android-loop)
- [The three commands that matter](#the-three-commands-that-matter)
- [How the UI is written](#how-the-ui-is-written)
- [The backend, in the same workspace](#the-backend-in-the-same-workspace)
- [Repository layout](#repository-layout)
- [Working in an IDE](#working-in-an-ide)
- [Building each platform](#building-each-platform)
- [The Android package](#the-android-package)
- [Tests and quality gates](#tests-and-quality-gates)
- [What is verified, and what is not](#what-is-verified-and-what-is-not)
- [Renaming it to your own app](#renaming-it-to-your-own-app)
- [Contributing, and reporting what you find](#contributing-and-reporting-what-you-find)
- [Author & contact](#author--contact)
- [Licence](#licence)

---

## Get the code

```bash
git clone git@github.com:Rustinaz/code-template.git
cd code-template
```

`Cargo.lock` is committed on purpose, so your first build resolves to exactly the dependency
versions the template was tested with.

If you would rather have a tarball than a clone, archive the directory without its build
output. `tar` matches `--exclude` against the *stored* path, so the excludes must be written
out in full relative to where the command runs — an exclude of `./target` silently matches
nothing when the archive is created from the parent directory:

```bash
tar --exclude='code-template/target' \
    --exclude='code-template/android-app/build' \
    --exclude='code-template/android-app/.gradle' \
    --exclude='code-template/apps/example/dist' \
    -czf code-template.tar.gz code-template
```

That produces a 200-odd kB archive. If it is tens of megabytes, the excludes did not match.

Never ship `target/` (many GB of build cache), `apps/example/dist/` (Trunk output) or
`android-app/build/`. All three are generated and all three are in `.gitignore`.

## Prerequisites

| To build | You need |
| --- | --- |
| Linux, Windows, macOS desktop | A Rust toolchain (`rustup`). Nothing else. |
| Android | The above, plus the Android SDK with an NDK (r23 or newer). Android Studio is the easy way. |
| iOS | macOS with Xcode. iOS 12 SDK or newer. |
| Web | The above, plus `cargo install --locked trunk`. |

The Rust targets for Android and the web:

```bash
rustup target add \
  aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android \
  wasm32-unknown-unknown
```

## Step 1: run it on Linux

```bash
cd code-template
cargo run --package example-app
```

A window opens with a Material 3 themed egui UI. Add `-d` for debug logging:

```bash
cargo run --package example-app -- --debug
cargo run --package example-app -- --help
```

The command line accepts `--platform`, `--framework`, `--debug/-d`, `--config/-c PATH` and
`--server URL`. `--server` points the client at the bundled backend and is explained
[below](#the-backend-in-the-same-workspace).

To run the client and its backend together, skip ahead to `scripts/dev.sh` in the same
section.

## Step 2: the desktop + Android loop

This is the workflow the layout is built around: edit once, see it on both.

**Terminal 1 — desktop, rebuilds and relaunches on every save**

```bash
cargo install cargo-watch          # once
cargo watch -x "run --package example-app"
```

**Terminal 2 — Android, rebuild and reinstall on every save**

```bash
source scripts/android-env.sh      # puts the NDK's clang where Cargo can find it
cargo watch -x "build --package example-app --target aarch64-linux-android"
```

That keeps `libexample_app.so` current. To get it onto the device, either re-run Gradle
(`./gradlew installDebug`) or push the rebuilt library and restart the activity:

```bash
adb install -r android-app/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.rustcrossplatform/android.app.NativeActivity
```

Building already-installed code again is the slow part, so a tight loop is usually: edit,
`cargo watch` rebuilds the `.so`, then re-run `./gradlew installDebug` when you actually want
to look at it on the device. For a single ABI that is a few seconds:

```bash
cd android-app && ./gradlew installDebug -PandroidAbi=arm64-v8a
```

**Terminal 3 — the web, optional**

```bash
scripts/build-web.sh --serve       # http://localhost:8080, rebuilds on save
```

`scripts/android-env.sh` only exports variables; it does not build anything. Sourcing it in
one terminal does not affect the others, which is why terminal 2 has to source it too.

**The backend, if you are using it.** `scripts/dev.sh` starts it next to the desktop client.
To run it on its own — for an emulator, say — use `cargo server` (short for
`cargo run --package server`); see [the backend section](#the-backend-in-the-same-workspace).

## The three commands that matter

```bash
# 1. Check everything, every crate and every target
cargo check --workspace --all-targets

# 2. Run the tests
cargo test --workspace --all-targets

# 3. Lint, with warnings treated as errors
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

And formatting:

```bash
cargo fmt --all
```

Short aliases are defined in `.cargo/config.toml`:

| Alias | Expands to |
| --- | --- |
| `cargo c` | `check --workspace --all-targets` |
| `cargo t` | `test --workspace --all-targets` |
| `cargo b` / `cargo br` | `build --workspace` / `--release` |
| `cargo f` | `fmt --all` |
| `cargo l` | `clippy --workspace --all-targets` |
| `cargo check-all` | `check --workspace --all-targets --all-features` |
| `cargo server` | `run --package server` |
| `cargo deps` | `tree --workspace --depth 2` |
| `cargo docs` | `doc --workspace --no-deps` |

None of them is named after a real Cargo subcommand, and that is deliberate: Cargo resolves
an alias whose name matches a built-in subcommand to *itself*, so `fmt = "fmt --all"` would
make `cargo fmt` recurse into `cargo fmt` forever. Hence `f` rather than `fmt`, and `l`
rather than `clippy`.

## Feature flags

Two features are worth knowing about. The first is **off by default**, and the default is the
one that compiles everywhere; the second is on by default and is what you turn off to drop
the renderer.

**`shared`'s `network` feature.** It gates the only dependency that is not pure Rust
(`reqwest`, which needs TLS) and therefore cannot cross-compile to every target without a C
toolchain. With it off, `shared` re-exports a `StubNetworkService` under the name
`DefaultNetworkService`, so calling code is identical either way and simply gets
`AppError::Unsupported` at runtime:

```bash
cargo build -p shared                        # stub, no TLS, cross-compiles everywhere
cargo build -p shared --features network     # real reqwest client
```

Turn it on in `apps/example/Cargo.toml` when your app actually makes HTTP calls. The bundled
client does **not** need it: its calls to `apps/server` go through the TLS-free
`HttpNetworkService`, which is built on `std` alone.

**`ui`'s `egui` feature** (on by default). Turning it off drops the renderer, which makes
the component crate usable from a platform crate that only wants tokens and trait
definitions:

```bash
cargo check -p ui --no-default-features
```

Every component is split along that seam, which the next section explains.

## How the UI is written

Every platform draws the same widgets with the same egui code. There is no per-platform UI
to keep in sync, because there is only one UI.

**The split you asked for — shared logic plus platform-specific pieces — is in the crates:**

```
crates/ui/          shared UI: theme tokens, layout maths, components, PlatformUi trait
crates/platform/*   one crate per platform: what actually needs the OS
```

`crates/ui/src/components/` is the shared component library. Every component follows the
same two-part shape:

1. **Pure logic** — the builder, the state struct, the sizing and radius arithmetic. No
   `egui` types at all, so it compiles and is unit tested with the `egui` feature off.
2. **Rendering** — the `show(&mut egui::Ui)` method, gated behind `#[cfg(feature = "egui")]`.

That is what lets `cargo check -p ui --no-default-features` succeed, and it is why the
platform crates can depend on `ui` for tokens and trait definitions without pulling in a
renderer.

A component looks like this:

```rust
pub struct Card {
    title: String,
    elevation: Elevation,
    padding: f32,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self { /* ... */ }

    pub fn elevation(mut self, elevation: Elevation) -> Self { /* ... */ }

    /// Draws the card. Only compiled with the `egui` feature.
    #[cfg(feature = "egui")]
    pub fn show(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style())
            .fill(self.background)
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.heading(&self.title);
            });
    }
}
```

**The platform crates own only what the OS actually has to be told.** Window title, theme
defaults, device information, accessibility announcements:

```rust
// crates/ui/src/platform/mod.rs
pub trait PlatformUi {
    fn platform(&self) -> Platform;
    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>>;
    fn theme(&self) -> PlatformTheme;
    fn announce_for_accessibility(&self, text: &str);
    // ...
}

// crates/platform/android/src/lib.rs
impl PlatformUi for AndroidPlatformUi {
    fn platform(&self) -> Platform { Platform::Android }
    fn theme(&self) -> PlatformTheme { material3_theme() }
    fn announce_for_accessibility(&self, text: &str) {
        // Real JNI call into android.view.View.announceForAccessibility.
    }
}
```

`UiBuilder` wires it together, and `.platform(Platform::Android)` is what corrects the
platform on a device, where the value baked into the config file is the build host's:

```rust
let context = UiBuilder::new()
    .platform(Platform::Android)      // the device, not the machine that compiled it
    .device_info(DeviceInfo::current())
    .build()?;
```

### The Android entry point

`android-activity` 0.6 requires a function with this exact signature, so the example app
defines it next to everything else, in `apps/example/src/lib.rs`:

```rust
#[cfg(target_os = "android")]
#[no_mangle]
#[allow(improper_ctypes_definitions)] // AndroidApp is a Rust handle, never passed from C
pub extern "C" fn android_main(app: platform_android::AndroidApp) {
    platform_android::install_app(app.clone());
    // ...
}
```

### The web entry point

The browser has no `main`, so eframe's `WebRunner` is started from a
`#[wasm_bindgen(start)]` hook instead. It finds its `<canvas>` by id, which is why
`index.html` must contain a matching element:

```rust
#[cfg(target_arch = "wasm32")]
pub const WEB_CANVAS_ID: &str = "the_canvas_id";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn run_web() -> Result<(), wasm_bindgen::JsValue> { /* ... */ }
```

The desktop `run()` and the wasm `run_web()` are mutually exclusive on purpose: `run_native`
needs an operating system window and `WebRunner` needs a document, so neither compiles for
the other's target, and each is `#[cfg]`-gated.

`src/main.rs` is a thin binary that calls `example_app::run()` on desktop and stubs out to
`unreachable!()` under wasm. Keeping the app in the library is what lets the same code be a
desktop binary, an Android `.so`, a browser `.wasm` and an iOS `.a`.

## The backend, in the same workspace

The template ships a backend as a normal workspace member (`apps/server`), not as a separate
`npm`-style project, so the server and the client are written and run in one place. It is
deliberately dependency-light: pure Rust, a thread per connection, blocking I/O and no async
runtime. That keeps it easy to read and, more importantly, keeps the whole workspace
cross-compiling without a C toolchain.

**The contract is written once.** `crates/shared/src/api/` holds the endpoint paths and the
`serde` request/response types:

```rust
// crates/shared/src/api/mod.rs
pub mod endpoints {
    pub const HEALTH: &str = "/api/health";
    pub const USERS: &str = "/api/users";
    pub const USERS_SEARCH: &str = "/api/users/search";
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RegisterRequest { /* ... */ }
```

`apps/server` imports this module and so does `apps/example`, so the two sides cannot drift:
rename a field and the other end stops compiling.

**The server** is hand-written too. `apps/server/src/http.rs` parses an HTTP/1.1 request and
writes a response, `apps/server/src/lib.rs` maps the `endpoints` constants to handlers, and
`main.rs` is a `clap` binary:

```bash
cargo run --package server                          # 127.0.0.1:8080, with demo users
cargo run --package server -- --bind 0.0.0.0:8080   # reachable from an emulator or the LAN
cargo run --package server -- --no-demo             # start with an empty user store
```

**The client** side is `shared::services::HttpNetworkService`, a small blocking HTTP/1.1
client built on `std::net`. It is `#[cfg(not(target_arch = "wasm32"))]` and speaks `http://`
only: dragging a TLS stack into a cross-compiled mobile build is not worth it for a local
backend. Because it does not use `reqwest`, it does not need `shared`'s `network` feature, and
the Android and web builds stay free of TLS.

**Run both with one command:**

```bash
scripts/dev.sh                                # backend on 127.0.0.1:8080, then the desktop client
scripts/dev.sh --release
scripts/dev.sh --bind 0.0.0.0:8080            # also accept emulator or LAN clients
scripts/dev.sh --server-only                  # just the backend, in the foreground
scripts/dev.sh --client-only --server http://127.0.0.1:8080
```

`dev.sh` starts the backend, waits until its port actually answers, launches the client
pointed at it with `--server`, and stops the backend when the client exits — Ctrl-C included.
That is the "written together, run together" loop: one terminal, one command.

The client's `--server URL` overrides `network.base_url` for one run. The built-in default is
already the bundled backend's address, so plain `cargo run --package example-app` finds it:

```bash
cargo run --package example-app -- --server http://192.168.1.20:8080
```

On Android, `127.0.0.1` is the device itself, so forward the host's port before launching the
app:

```bash
adb reverse tcp:8080 tcp:8080    # the device's localhost:8080 -> this machine
```

The example app draws a small **Backend** panel that can call each endpoint without blocking
the UI: requests run on a worker thread and post their results back over an `mpsc` channel, so
egui never stalls on the network. A browser has no raw sockets, so the web build renders an
explanatory placeholder rather than pretending to connect; a real web app would call the same
endpoints through `web_sys::fetch`.

The HTTP/1.1 wire code is hand-written on purpose. When you outgrow it, the seam is one file:
routing in `apps/server/src/lib.rs`, where a framework such as `axum` slots in without the
DTOs or the client changing at all.

## Repository layout

```
rust-crossplatform-template/
├── Cargo.toml                  workspace manifest: members, shared dependency versions
├── Cargo.lock                  committed, on purpose
├── rust-toolchain.toml         pins stable + rustfmt/clippy so editors and CI agree
├── .editorconfig               editor-neutral UTF-8 / LF / indentation rules
├── .cargo/config.toml          network settings and short aliases; no hard-coded linkers
│
├── apps/
│   ├── example/                the app you will actually edit
│   │   ├── src/lib.rs          run(), run_web(), android_main(), the UI
│   │   ├── src/backend.rs      the client's backend panel (native) and web placeholder
│   │   ├── src/main.rs         thin desktop binary
│   │   ├── index.html          the web shell; holds the <canvas> eframe mounts
│   │   ├── Trunk.toml          web build config
│   │   └── dist/               Trunk output (generated, gitignored)
│   └── server/                 the bundled backend, pure Rust
│       ├── src/lib.rs          routing: endpoint constants -> handlers
│       ├── src/http.rs         HTTP/1.1 request parsing and response writing
│       └── src/main.rs         clap binary (`--bind`, `--no-demo`)
│
├── crates/
│   ├── shared/                 business logic: domain, config, errors, services
│   │   ├── src/api/            the wire contract shared with apps/server
│   │   └── src/services/       repositories, auth, settings, network (+ http client, stub)
│   ├── ui/                     theme, layout, navigation, components, PlatformUi
│   │   └── src/components/     the shared component library
│   ├── resources/              strings, colours, themes, assets, loaded at runtime
│   ├── build-config/           version, target and signing configuration
│   └── platform/
│       ├── linux/              window, theme, device info
│       ├── android/            JNI, device info, activity control
│       ├── ios/                device info, window, haptics
│       ├── windows/            window, theme, device info
│       ├── macos/              window, theme, device info
│       └── web/                user agent, viewport, touch detection
│
├── android-app/                the Android Studio project. No Java/Kotlin source, no layouts.
│   ├── settings.gradle.kts
│   ├── app/build.gradle.kts    runs Cargo per ABI, then packages the .so
│   ├── app/src/main/
│   │   ├── AndroidManifest.xml points at android.app.NativeActivity
│   │   └── res/               strings, theme, launcher icon
│   ├── gradlew                 the wrapper
│   └── proguard-rules.pro
│
├── resources/                  shared data: strings_en.json, strings_es.json, colors.json
├── scripts/                    one build script per platform
└── .github/workflows/ci.yml    six jobs covering all six targets plus the backend
```

The dependency direction is strictly one way:

```
apps/example  ->  ui  ->  shared  <-  apps/server
       |           |        ^
       |           +-> platform/linux, /windows, /macos, /android, /ios, /web
       +-> resources
```

`shared` and `ui` never depend on a platform crate, which is why `cargo check --workspace`
succeeds on a machine that has no SDKs installed at all. `apps/server` depends on `shared`
alone — it never touches `ui` — so the backend carries no GUI code into the server binary.

## Working in an IDE

The project is editor-agnostic on purpose: your personal editor state is not committed (most
of `.vscode/` and all of `.idea/` is in `.gitignore`), so you open the folder in whatever
editor you use and the workspace resolves itself. A few small, committed files make every
editor behave the same way:

- `rust-toolchain.toml` pins `stable` with the `rustfmt` and `clippy` components, so your
  editor's analysis and your terminal use the same toolchain — the one CI uses too.
- `.editorconfig` sets UTF-8, LF and 4-space indentation (2 for TOML/YAML). VS Code,
  RustRover, Zed, Neovim and Android Studio all read it natively. It agrees with `rustfmt`;
  the formatter, not the editor, is what decides Rust layout.
- `.vscode/` holds four shared files — a run configuration, tasks, rust-analyzer settings and
  recommended extensions — so a VS Code user can run the app with no setup. Other editors
  ignore the directory entirely.

**Editing Rust.** Open the repository root in an editor with rust-analyzer (VS Code, Zed,
Neovim) or in RustRover. It finds the workspace, all twelve crates and every test with no
generation step. One caveat: code behind `#[cfg(target_os = "android")]` is not
type-checked by default, because rust-analyzer analyzes for your host. To cover it, point
rust-analyzer at an Android target — in VS Code, in your own uncommitted `.vscode/`:

```json
{
  "rust-analyzer.cargo.target": "aarch64-linux-android"
}
```

and switch it back for desktop work. Either way, the cross-compile in the quality-gate
section is what actually proves that code.

**`ui`'s feature split.** `ui`'s `egui` feature is on by default. If the editor analyzes it
with `--no-default-features`, the rendering methods drop out of analysis — that is expected,
and `cargo check -p ui --no-default-features` is the honest check.

**Android packaging.** Open `android-app/` in Android Studio to sync, run and install the APK
(`./gradlew installDebug`). It is the packaging IDE, not where the app is written: there is no
Kotlin or Java to edit and no layout XML, and the Rust is edited in the Rust editor above.
Android Studio has no rust-analyzer, so keep two windows — the Rust code on one side, the
device and Logcat on the other.

**Running from VS Code.** With the committed configuration, `F5` builds and launches the
desktop app under the debugger. The app's default backend URL is already
`http://127.0.0.1:8080`, so start the backend first — the *Rustinaz: backend (server)* task, or
`scripts/dev.sh` in a terminal — or pick the *Rustinaz: run example-app against the backend*
configuration. The *Rustinaz: server + client (dev.sh)* task starts both in one terminal if you
would rather not use the debugger at all.

## Building each platform

```bash
scripts/build-all.sh --list      # what this machine can build
scripts/build-all.sh             # build all of them
scripts/build-all.sh android web # just these
```

Individually:

```bash
scripts/build-linux.sh [--release]
scripts/build-android.sh [--release] [--abi arm64-v8a] [--apk]
scripts/build-windows.sh [--release]     # needs MinGW-w64, or run on Windows
scripts/build-macos.sh [--release]       # macOS only
scripts/build-ios.sh [--simulator]       # macOS only
scripts/build-web.sh [--serve]           # needs trunk
scripts/dev.sh [--release] [--bind ADDR] # run the backend and the client together
```

### Android without Gradle

If you only want the `.so`, skip Gradle entirely:

```bash
source scripts/android-env.sh
cargo build --package example-app --target aarch64-linux-android --release
# -> target/aarch64-linux-android/release/libexample_app.so
```

`scripts/android-env.sh` is the reason this works without editing any config. NDK r23 and
newer removed the bare `<arch>-linux-android-clang` symlinks, so both the Rust linker and
the C compiler that `cc-rs` needs have to be named explicitly. The script exports
`CARGO_TARGET_<TRIPLE>_LINKER`, `CC_<triple>`, `CXX_<triple>`, `AR_<triple>` and
`RANLIB_<triple>` for all four ABIs, and the `cargoBuild` Gradle task exports exactly the
same set.

## The Android package

`android-app/` is a normal Android Studio project that happens to contain no source code.

- `AndroidManifest.xml` declares `android.app.NativeActivity` and
  `android.app.lib_name = "example_app"`. That is the whole contract with Android: load the
  library, let it take over the window.
- `app/build.gradle.kts` runs `cargo build --target <triple>` once per ABI, stages the
  results in `build/rustJniLibs/<abi>/`, and lets AGP package them as `jniLibs`.
- The `res/` directory holds a theme, two string entries and a launcher icon. That is all
  the resource configuration a full-screen native surface needs.

One non-obvious setting: `packaging { jniLibs { keepDebugSymbols += "**/*.so" } }`. AGP runs
its own ELF stripper over every packaged native library by default, and on a cross-compiled
Rust cdylib it can mistake the file for something it understands and emit a truncated stub
in its place — an APK that installs and then crashes on start-up. Stripping is Cargo's job
(`[profile.release]` sets `strip = "symbols"`), because Cargo uses the same LLVM toolchain
that produced the object. Debug builds keep their symbols so the NDK debugger can
symbolicate native crashes.

```bash
cd android-app
./gradlew assembleDebug                       # all four ABIs
./gradlew assembleDebug -PandroidAbi=arm64-v8a # one ABI, much faster
./gradlew installDebug                         # build and install on the connected device
```

`adb install -r app/build/outputs/apk/debug/app-debug.apk`

The manifest sets `windowSoftInputMode="adjustResize"`, so the Android keyboard resizes the
surface instead of covering it, and egui's text input follows the new size.

## Tests and quality gates

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Because `shared`'s `network` feature is off by default, **`--all-features` is not optional**
for a full check: without it, the real `DefaultNetworkService` is never compiled and the
stub is tested in its place. The `cargo check-all` alias is the convenient way to ask for
everything:

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
```

Every crate has unit tests. The component tests render a frame into an `egui::Context` and
assert on the result, which catches panics and geometry errors without a display server.
The platform crates' tests run on any host because each platform crate compiles a stub of
its OS-specific handle for non-native targets.

The `ui` crate is checked both ways, because the feature split is the point:

```bash
cargo check -p ui                       # with egui
cargo check -p ui --no-default-features # logic only, no renderer
```

`cargo test` on a host does **not** exercise `#[cfg(target_os = "android")]` code. The only
way to check that layer is to cross-compile it:

```bash
source scripts/android-env.sh
cargo build --package example-app --target aarch64-linux-android
```

## What is verified, and what is not

Being explicit about this, because a template that quietly claims more than it delivers is
worse than a small one that tells the truth.

**Built and run here, from Linux:**

- `cargo check --workspace --all-targets --all-features` and
  `cargo test --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
  `cargo fmt --all --check`, both clean
- Linux desktop: `cargo run --package example-app` builds and runs. It was also launched with
  `--server http://127.0.0.1:8080` while the backend was up, through `scripts/dev.sh`.
- **The backend**: `cargo run --package server` was started and `curl` reached
  `/api/health`, `/api/users`, `/api/users/search?q=…` and `POST /api/users`, all served from
  the shared `api` contract. `scripts/dev.sh` starts the backend and the client together and
  stops the backend on exit.
- **Android**: a real `libexample_app.so` for all four ABIs, each verified with `file` and
  `llvm-nm` to export both `ANativeActivity_onCreate` and `android_main`. An APK was
  assembled by Gradle and inspected: it contains all four libraries, **zero `.class` files**
  (the manifest really does point at the framework's `NativeActivity`), and the
  `android.app.lib_name` value matches the library.
- **The web target**: built end to end with Trunk. `apps/example/dist/` contains a real
  `.wasm`, its wasm-bindgen glue and an `index.html` with the canvas and start-up script
  injected.

**Not verifiable on this machine, and honestly marked as such:**

- **iOS and macOS** need macOS and Xcode. The `crates/platform/ios` and
  `crates/platform/macos` crates compile as part of the workspace check, but nothing links
  an `.a` or a `.app` here. Note that Xcode 10.1, the newest version installable on macOS
  High Sierra, ships the iOS 12 SDK; that is the floor and the scripts target it.
- **Windows** needs MinGW-w64 to cross-compile, or a Windows host to build natively. This
  machine has neither, and no `sudo` to install one.
- **Nothing has been run on a physical Android device or an emulator from here.** The APK
  is well-formed and its contents were inspected, but "packages correctly" and "runs
  correctly" are different claims, and only the first was tested.
- **The backend panel's button was not clicked by a human here.** Its network path is
  `HttpNetworkService`, whose live-socket round trip is unit-tested, and its worker-thread
  plumbing compiles for every target, but the click-to-result path through the GUI was not
  exercised. The web build cannot reach a backend at all (a browser has no raw sockets) and
  says so on screen.

The CI workflow in `.github/workflows/ci.yml` covers all six targets plus the backend with six
jobs, and is the place where the unverified targets are actually proven.

## Renaming it to your own app

Six places, all mechanical:

1. `Cargo.toml` — `repository`, `authors`, `description` under `[workspace.package]`.
2. `apps/example/Cargo.toml` — the package name `example-app`, and `[lib] name`, which must
   stay `example_app` unless you also change `rustLibName` in the Gradle file and
   `android.app.lib_name` in the manifest. All three must agree.
3. `android-app/app/build.gradle.kts` — `rustPackage` and `rustLibName`.
4. `android-app/app/src/main/AndroidManifest.xml` — `android:name` and `android.app.lib_name`.
5. `android-app/app/build.gradle.kts` — `namespace` and `applicationId`.
6. `scripts/build-*.sh` and the CI workflow — the `example-app` package name.
7. `apps/server/` (optional) — if you rename the backend package, update `scripts/dev.sh`,
   which builds `--package server` and runs `$BIN_DIR/server`, and the `server` alias in
   `.cargo/config.toml`.

`libexample_app.so` is the Android native library name that ends up in the APK. It is
derived from `[lib] name`, not from the package name, which is why the two are set
separately.

## Contributing, and reporting what you find

This is a template meant to be exercised, broken and improved. If you clone it:

1. Run the gates in [Tests and quality gates](#tests-and-quality-gates) on **your** machine.
2. Try the target you actually care about — especially the ones that are only *compiled* here:
   Android on a real device, Windows, iOS and macOS.
3. When something fails, open an issue with the exact command, the full error, your OS, your
   toolchain (`rustc -V` and `cargo -V`) and, for Android, the NDK version.

Report findings rather than silently working around them — that is how the
"verified / not verified" list above gets shorter over time.

## Author & contact

**Rustinaz** is designed and maintained by **Sina Khanzadeh**.

- Résumé / website: [sina-khanzadeh.ir](https://sina-khanzadeh.ir)
- Email: [khanzadeh.1377@gmail.com](mailto:khanzadeh.1377@gmail.com)
- Instagram / Telegram / Discord: `programmer_1998`
  — [Instagram](https://instagram.com/programmer_1998) · [Telegram](https://t.me/programmer_1998)
- GitHub: [@programmer-1998](https://github.com/programmer-1998)

## Licence

MIT OR Apache-2.0, your choice.

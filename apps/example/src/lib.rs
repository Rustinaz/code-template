//! The example application.
//!
//! Everything lives in the library so that the *same* code path produces the
//! desktop binary and the Android `.so`:
//!
//! * desktop / Windows / macOS: `src/main.rs` calls [`run`],
//! * Android: `libexample_app.so` is loaded by `android.app.NativeActivity`,
//!   which calls [`android_main`] below.
//!
//! The UI is egui in both cases, which is what makes the component library in
//! `ui::components` genuinely shared rather than merely duplicated.

use std::sync::Arc;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use parking_lot::RwLock;
use resources::ResourceManager;
use shared::config::AppConfig;
use shared::{domain::Platform, init_blocking};
use tracing::info;
use ui::{UiBuilder, UiContext, UiFramework};

mod backend;

/// Window title used on every desktop target.
pub const WINDOW_TITLE: &str = "Rust CrossPlatform Example";

/// Default desktop window size, in logical pixels.
///
/// Desktop only: a browser window is sized by its own viewport, and
/// `canvas { width: 100vw }` in `index.html` is what fills it.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];

/// Default desktop minimum window size, in logical pixels.
#[cfg(not(target_arch = "wasm32"))]
const MIN_WINDOW_SIZE: [f32; 2] = [320.0, 240.0];

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

/// Builds the command-line interface used by the desktop binary.
#[must_use]
pub fn build_cli() -> Command {
    Command::new("example-app")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Cross-platform Rust example app")
        .arg(
            Arg::new("platform")
                .long("platform")
                .help("Platform to run on (auto-detected if not specified)")
                .value_name("PLATFORM")
                .num_args(1),
        )
        .arg(
            Arg::new("framework")
                .long("framework")
                .help("UI framework to use")
                .value_name("FRAMEWORK")
                .num_args(1),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug logging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Config file path")
                .value_name("PATH")
                .num_args(1),
        )
        .arg(
            Arg::new("server")
                .long("server")
                .help("Base URL of the bundled backend in apps/server")
                .value_name("URL")
                .num_args(1),
        )
}

/// The four things the app needs from the command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliOptions {
    /// Overrides the detected platform.
    pub platform: Option<Platform>,
    /// Overrides the UI framework.
    pub framework: Option<UiFramework>,
    /// Enables debug logging.
    pub debug: bool,
    /// Explicit config file path.
    pub config_path: Option<String>,
    /// Base URL of the backend, overriding `network.base_url`.
    pub server: Option<String>,
}

/// Extracts [`CliOptions`] from parsed arguments, ignoring unknown values.
#[must_use]
pub fn parse_args(matches: &ArgMatches) -> CliOptions {
    let platform =
        matches
            .get_one::<String>("platform")
            .and_then(|s| match s.to_lowercase().as_str() {
                "android" => Some(Platform::Android),
                "ios" => Some(Platform::Ios),
                "linux" => Some(Platform::Linux),
                "windows" => Some(Platform::Windows),
                "macos" => Some(Platform::Macos),
                "web" => Some(Platform::Web),
                _ => None,
            });

    let framework =
        matches
            .get_one::<String>("framework")
            .and_then(|s| match s.to_lowercase().as_str() {
                "egui" => Some(UiFramework::Egui),
                "iced" => Some(UiFramework::Iced),
                "slint" => Some(UiFramework::Slint),
                "tauri" => Some(UiFramework::Tauri),
                "native" => Some(UiFramework::Native),
                _ => None,
            });

    CliOptions {
        platform,
        framework,
        debug: matches.get_flag("debug"),
        config_path: matches.get_one::<String>("config").cloned(),
        server: matches.get_one::<String>("server").cloned(),
    }
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

/// Boots shared state, the platform layer, resources and the UI context.
///
/// This is the only part of start-up that differs per target, and it is
/// deliberately synchronous so both [`run`] and [`android_main`] can call it.
pub fn bootstrap(options: &CliOptions) -> Result<(UiContext, Arc<ResourceManager>)> {
    let mut config = match &options.config_path {
        Some(path) => {
            let content = std::fs::read_to_string(path)?;
            toml::from_str::<AppConfig>(&content)?
        }
        // `load` also publishes the config as the process-wide instance.
        None => AppConfig::load()?.as_ref().clone(),
    };

    // The config file records the machine that wrote it, so the running target
    // always wins unless the caller asked for something specific.
    if let Some(platform) = options.platform {
        config.platform.platform = platform;
    }

    // `--server` wins over both the config file and the built-in default, so a
    // single flag is enough to point the app at a backend on another host.
    if let Some(server) = &options.server {
        config.network.base_url = server.clone();
    }

    let config = init_blocking(Some(Arc::new(config)))?;
    init_platform(&config)?;
    let resources = ResourceManager::init_global(config.clone());

    info!(
        "Build: {} ({} profile)",
        env!("CARGO_PKG_VERSION"),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );

    let platform = config.platform.platform;
    let framework = options
        .framework
        .unwrap_or_else(|| default_framework(platform));

    let ui_context = UiBuilder::new(Arc::new(RwLock::new((*config).clone())))
        .platform(platform)
        .framework(framework)
        .build();

    info!("UI framework: {framework:?}");
    info!("Platform: {:?}", ui_context.platform());
    info!(
        "Device: {:?} ({:?})",
        ui_context.device_info.form_factor, ui_context.device_info.screen_size
    );

    Ok((ui_context, resources))
}

/// egui is the single UI backend in this template, so it is also the right
/// default on Android and iOS.
fn default_framework(_platform: Platform) -> UiFramework {
    UiFramework::Egui
}

/// Calls the matching `init()` on the platform crate for the target platform.
fn init_platform(config: &AppConfig) -> Result<()> {
    // Each platform crate returns the shared error type, which is `Send + Sync`,
    // so it converts into `anyhow::Error` without a manual stringification.
    match config.platform.platform {
        Platform::Android => {
            #[cfg(target_os = "android")]
            platform_android::init().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            platform_linux::init().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Platform::Windows => {
            #[cfg(target_os = "windows")]
            platform_windows::init().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Platform::Macos => {
            #[cfg(target_os = "macos")]
            platform_macos::init().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Platform::Ios | Platform::Web => {
            // No platform crate needs explicit set-up on these targets.
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Desktop entry point
// ---------------------------------------------------------------------------
//
// `eframe` splits its entry point in two: `run_native` needs an operating system
// window, and `WebRunner` needs a `<canvas>` in a document. Neither compiles
// for the other's target, so both are gated and each target gets exactly one.

/// Runs the app on a desktop target. Called by `src/main.rs`.
///
/// Not compiled for wasm: a browser has no `main` to hand this to.
#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> Result<()> {
    let matches = build_cli().get_matches();
    let options = parse_args(&matches);
    let (ui_context, _resources) = bootstrap(&options)?;
    run_egui(ui_context, eframe::NativeOptions::default())
}

/// The eframe/egui options shared by every target.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_title(WINDOW_TITLE),
        ..Default::default()
    }
}

/// Starts the egui event loop with `ui_context`.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_egui(ui_context: UiContext, options: eframe::NativeOptions) -> Result<()> {
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|_cc| Ok(Box::new(ExampleApp::new(ui_context)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

// ---------------------------------------------------------------------------
// Web entry point
// ---------------------------------------------------------------------------

/// The `id` of the `<canvas>` element the app mounts into.
///
/// Must match the element in `apps/example/index.html`. eframe cannot guess it:
/// it creates and owns its own canvas, but reads this one from the DOM.
#[cfg(target_arch = "wasm32")]
pub const WEB_CANVAS_ID: &str = "the_canvas_id";

/// Starts the app in the browser.
///
/// Trunk turns this `#[wasm_bindgen(start)]` function into the module's
/// start hook, so it runs as soon as the `.wasm` is instantiated.
///
/// The returned future is driven by eframe's own `requestAnimationFrame` loop,
/// which means this must never block.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
#[allow(clippy::unused_async)]
pub async fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::prelude::*;

    // No clap: there is no command line in a browser. The platform is known.
    let (ui_context, _resources) = bootstrap(&CliOptions {
        platform: Some(Platform::Web),
        ..CliOptions::default()
    })
    .map_err(|error| wasm_bindgen::JsValue::from_str(&format!("bootstrap failed: {error:#}")))?;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("no document"))?;

    // eframe wants an `HtmlCanvasElement`, and `get_element_by_id` can only
    // promise that it is *some* `Element`, so the cast is checked: a div with
    // the right id would otherwise fail deep inside WebGL setup.
    let canvas = document
        .get_element_by_id(WEB_CANVAS_ID)
        .ok_or_else(|| {
            wasm_bindgen::JsValue::from_str(&format!("no element with id {WEB_CANVAS_ID:?}"))
        })?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| {
            wasm_bindgen::JsValue::from_str(&format!("{WEB_CANVAS_ID:?} is not a <canvas>"))
        })?;

    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|_cc| Ok(Box::new(ExampleApp::new(ui_context)))),
        )
        .await
}

// ---------------------------------------------------------------------------
// Android entry point
// ---------------------------------------------------------------------------

/// Entry point called by `android.app.NativeActivity` after it has loaded
/// `libexample_app.so`.
///
/// `android-activity` spawns this on its own thread, so nothing here may
/// assume it runs on the JVM main thread and nothing here may await.
#[cfg(target_os = "android")]
#[no_mangle]
// `AndroidApp` is a Rust-side handle to the `JavaVM`, passed in by
// `android-activity` from Rust. It is never constructed or inspected by C, so
// its layout does not need to be FFI-stable. `extern "C"` is kept because it is
// the entry-point convention `android-activity` documents and looks up.
#[allow(improper_ctypes_definitions)]
pub extern "C" fn android_main(app: platform_android::AndroidApp) {
    // Publish the handle so worker threads can reach the JVM later on
    // (accessibility announcements, for example).
    platform_android::install_app(app.clone());

    let info = platform_android::device_info();
    tracing::info!(
        "Android: {} (API {}, model {}, abi {})",
        info.version_release,
        info.api_level,
        info.model,
        info.supported_abis
    );

    let ui_context = match bootstrap(&CliOptions {
        platform: Some(Platform::Android),
        ..CliOptions::default()
    }) {
        Ok((ui_context, _resources)) => ui_context,
        Err(error) => {
            tracing::error!("bootstrap failed: {error:#}");
            return;
        }
    };

    // On Android the `AndroidApp` is *not* global state, so it has to be
    // handed to winit explicitly before the event loop is built.
    let mut options = native_options();
    options.event_loop_builder = Some(Box::new(move |builder| {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        builder.with_android_app(app);
    }));

    if let Err(error) = run_egui(ui_context, options) {
        tracing::error!("eframe error: {error:#}");
    }
}

// ---------------------------------------------------------------------------
// The app itself
// ---------------------------------------------------------------------------

/// The example application state.
struct ExampleApp {
    /// Shared UI context: config, theme tokens, device info.
    ui_context: UiContext,
    /// Demo counter value.
    counter: i32,
    /// Demo text field value.
    name: String,
    /// Whether the settings window is open.
    show_settings: bool,
    /// Live client for the bundled backend.
    backend: backend::BackendPanel,
}

impl ExampleApp {
    /// Creates the app from an already-booted [`UiContext`].
    fn new(ui_context: UiContext) -> Self {
        let backend = {
            let config = ui_context.config.read();
            backend::BackendPanel::new(
                config.network.base_url.clone(),
                std::time::Duration::from_millis(config.network.timeout_ms),
            )
        };

        Self {
            ui_context,
            counter: 0,
            name: "User".to_string(),
            show_settings: false,
            backend,
        }
    }

    fn config_read(&self) -> parking_lot::RwLockReadGuard<'_, AppConfig> {
        self.ui_context.config.read()
    }

    fn config_write(&mut self) -> parking_lot::RwLockWriteGuard<'_, AppConfig> {
        self.ui_context.config.write()
    }

    /// Copies the Material 3 tokens from the shared theme into egui's visuals.
    fn apply_theme(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui;
        use shared::config::ThemeMode;

        let config = self.config_read();
        let tokens = &self.ui_context.theme_tokens;

        let color = |role: &shared::domain::ColorRole, dark: bool| {
            let value = if dark { &role.dark } else { &role.light };
            egui::Color32::from_rgb(value.rgb.0, value.rgb.1, value.rgb.2)
        };

        let mut visuals = egui::Visuals::default();
        match config.ui.theme.mode {
            ThemeMode::Light => {
                visuals.override_text_color = Some(color(&tokens.colors.on_background, false));
                visuals.panel_fill = color(&tokens.colors.background, false);
                visuals.window_fill = color(&tokens.colors.surface, false);
            }
            ThemeMode::Dark => {
                visuals.override_text_color = Some(color(&tokens.colors.on_background, true));
                visuals.panel_fill = color(&tokens.colors.background, true);
                visuals.window_fill = color(&tokens.colors.surface, true);
                visuals.dark_mode = true;
            }
            ThemeMode::HighContrast => {
                // High contrast is dark with pure-white text.
                visuals.override_text_color = Some(egui::Color32::WHITE);
                visuals.panel_fill = egui::Color32::BLACK;
                visuals.window_fill = egui::Color32::BLACK;
                visuals.dark_mode = true;
            }
            ThemeMode::System => {
                visuals.dark_mode = ctx.style().visuals.dark_mode;
            }
        }
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for ExampleApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use eframe::egui;

        self.apply_theme(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(WINDOW_TITLE);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                self.draw_platform_info(ui);
                ui.add_space(16.0);
                self.draw_counter(ui);
                ui.add_space(16.0);
                self.draw_text_input(ui);
                ui.add_space(16.0);
                self.draw_theme_picker(ui);
                ui.add_space(16.0);
                self.backend.show(ui);
            });
        });

        self.draw_settings(ctx);
    }
}

impl ExampleApp {
    fn draw_platform_info(&self, ui: &mut eframe::egui::Ui) {
        use eframe::egui;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label("Platform");
            ui.separator();
            egui::Grid::new("platform_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Platform");
                    ui.label(format!("{:?}", self.ui_context.platform()));
                    ui.end_row();
                    ui.label("Form factor");
                    ui.label(format!("{:?}", self.ui_context.device_info.form_factor));
                    ui.end_row();
                    ui.label("Screen");
                    ui.label(format!("{:?}", self.ui_context.device_info.screen_size));
                    ui.end_row();
                    ui.label("Theme");
                    ui.label(format!("{:?}", self.ui_context.theme_preference()));
                    ui.end_row();
                });
        });
    }

    fn draw_counter(&mut self, ui: &mut eframe::egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label("Counter");
            ui.separator();
            ui.label(format!("Count: {}", self.counter));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("➖ Decrement").clicked() {
                    self.counter = self.counter.saturating_sub(1);
                }
                if ui.button("➕ Increment").clicked() {
                    self.counter = self.counter.saturating_add(1);
                }
                if ui.button("🔄 Reset").clicked() {
                    self.counter = 0;
                }
            });
        });
    }

    fn draw_text_input(&mut self, ui: &mut eframe::egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label("Text input");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.name);
            });
            ui.label(format!("Hello, {}!", self.name));
        });
    }

    fn draw_theme_picker(&mut self, ui: &mut eframe::egui::Ui) {
        use shared::config::ThemeMode;

        let current = self.config_read().ui.theme.mode;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label("Theme");
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for theme in [
                    ThemeMode::System,
                    ThemeMode::Light,
                    ThemeMode::Dark,
                    ThemeMode::HighContrast,
                ] {
                    let selected = current == theme;
                    if ui
                        .selectable_label(selected, format!("{theme:?}"))
                        .clicked()
                    {
                        self.config_write().ui.theme.mode = theme;
                    }
                }
            });
        });
    }

    fn draw_settings(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui;

        if !self.show_settings {
            return;
        }
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                let mut config = self.config_write();
                ui.label("UI");
                ui.add(egui::Slider::new(&mut config.ui.ui_scale, 0.5..=3.0).text("UI scale"));
                ui.add(egui::Slider::new(&mut config.ui.font_scale, 0.5..=3.0).text("Font scale"));
                ui.separator();
                ui.label("Accessibility");
                ui.checkbox(&mut config.ui.reduced_motion, "Reduced motion");
                ui.checkbox(&mut config.ui.high_contrast, "High contrast");
                ui.separator();
                if ui.button("💾 Save").clicked() {
                    if let Err(error) = config.save() {
                        tracing::warn!("could not save settings: {error:#}");
                    }
                }
            });
        self.show_settings = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> CliOptions {
        let mut argv = vec!["example-app"];
        argv.extend_from_slice(args);
        parse_args(&build_cli().try_get_matches_from(argv).unwrap())
    }

    #[test]
    fn defaults_are_empty() {
        assert_eq!(options(&[]), CliOptions::default());
    }

    #[test]
    fn debug_flag_is_parsed() {
        assert!(options(&["--debug"]).debug);
        assert!(options(&["-d"]).debug);
    }

    #[test]
    fn every_known_platform_is_parsed() {
        for (name, expected) in [
            ("android", Platform::Android),
            ("ios", Platform::Ios),
            ("linux", Platform::Linux),
            ("windows", Platform::Windows),
            ("macos", Platform::Macos),
            ("web", Platform::Web),
        ] {
            let parsed = options(&["--platform", name]);
            assert_eq!(parsed.platform, Some(expected), "--platform {name}");
        }
    }

    #[test]
    fn platform_is_case_insensitive() {
        assert_eq!(
            options(&["--platform", "ANDROID"]).platform,
            Some(Platform::Android)
        );
    }

    #[test]
    fn unknown_platform_falls_back_to_none() {
        assert_eq!(options(&["--platform", "symbian"]).platform, None);
    }

    #[test]
    fn every_known_framework_is_parsed() {
        for (name, expected) in [
            ("egui", UiFramework::Egui),
            ("iced", UiFramework::Iced),
            ("slint", UiFramework::Slint),
            ("tauri", UiFramework::Tauri),
            ("native", UiFramework::Native),
        ] {
            let parsed = options(&["--framework", name]);
            assert_eq!(parsed.framework, Some(expected), "--framework {name}");
        }
    }

    #[test]
    fn config_path_is_parsed() {
        assert_eq!(
            options(&["-c", "/tmp/app.toml"]).config_path.as_deref(),
            Some("/tmp/app.toml")
        );
        assert_eq!(
            options(&["--config", "/tmp/app.toml"])
                .config_path
                .as_deref(),
            Some("/tmp/app.toml")
        );
    }

    #[test]
    fn server_url_is_parsed() {
        assert_eq!(
            options(&["--server", "http://127.0.0.1:9000"])
                .server
                .as_deref(),
            Some("http://127.0.0.1:9000")
        );
        assert_eq!(options(&[]).server, None);
    }

    #[test]
    fn all_options_together() {
        let parsed = options(&[
            "-d",
            "--platform",
            "web",
            "--framework",
            "egui",
            "-c",
            "a.toml",
        ]);
        assert!(parsed.debug);
        assert_eq!(parsed.platform, Some(Platform::Web));
        assert_eq!(parsed.framework, Some(UiFramework::Egui));
        assert_eq!(parsed.config_path.as_deref(), Some("a.toml"));
    }

    #[test]
    fn egui_is_the_default_framework_on_every_platform() {
        for platform in [
            Platform::Linux,
            Platform::Windows,
            Platform::Macos,
            Platform::Android,
            Platform::Ios,
            Platform::Web,
        ] {
            assert_eq!(default_framework(platform), UiFramework::Egui);
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        build_cli().debug_assert();
    }
}

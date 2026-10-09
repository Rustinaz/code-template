//! Android platform integration.
//!
//! The UI is rendered entirely by Rust (egui/eframe), so this crate only owns
//! what genuinely needs the Android runtime:
//!
//! * the [`AndroidApp`] handle produced by `android_main`,
//! * device / OS information read through JNI,
//! * an [`AndroidPlatformUi`] adapter implementing [`ui::platform::PlatformUi`],
//! * the Material 3 theme defaults.
//!
//! The APK itself is assembled by Gradle from the `android-app/` directory in
//! the repository root. There is no Java or Kotlin code in the app: the
//! manifest points at the framework's own `android.app.NativeActivity`, so the
//! only thing Java exists for is letting Android find a window.

use std::sync::OnceLock;

use parking_lot::RwLock;
use shared::domain::Platform;
use shared::errors::Result as AppResult;
use ui::platform::{PlatformTheme, PlatformUi, Window, WindowConfig};

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;

/// Stand-in for the Android-only handle so the crate's API is identical on
/// every host. Nothing can ever be installed into it off-device.
#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone)]
pub struct AndroidApp;

#[cfg(not(target_os = "android"))]
impl AndroidApp {
    /// Placeholder constructor; see [`current_app`] for the real lookup.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self
    }
}

/// Static information about the Android device the app is running on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AndroidInfo {
    /// `Build.VERSION.SDK_INT`, the Android API level (e.g. 34).
    pub api_level: i32,
    /// `Build.VERSION.RELEASE`, e.g. `"14"`.
    pub version_release: String,
    /// `Build.DISPLAY`, e.g. `"Pixel 8"`.
    pub display: String,
    /// `Build.MANUFACTURER`, e.g. `"Google"`.
    pub manufacturer: String,
    /// `Build.MODEL`, e.g. `"Pixel 8"`.
    pub model: String,
    /// `Build.SUPPORTED_ABIS` joined with `,`.
    pub supported_abis: String,
    /// ABI this binary was compiled for, from `std::env::consts::ARCH`.
    pub compiled_arch: &'static str,
}

impl AndroidInfo {
    /// Returns `true` when the device runs Android 10 (API 29) or newer.
    #[must_use]
    pub const fn is_android_10_or_newer(&self) -> bool {
        self.api_level >= 29
    }
}

static APP: OnceLock<AndroidApp> = OnceLock::new();

/// Publishes the `AndroidApp` handed to `android_main` so that any thread can
/// reach the JNI environment afterwards.
///
/// Must be called before [`current_app`] is used; calling it twice is a no-op.
pub fn install_app(app: AndroidApp) {
    let _ = APP.set(app);
}

/// Returns the installed `AndroidApp`, if `android_main` has run.
#[must_use]
pub fn current_app() -> Option<&'static AndroidApp> {
    APP.get()
}

/// Initializes the Android platform layer.
///
/// Safe to call on a desktop host too, where it only logs.
pub fn init() -> AppResult<()> {
    tracing::info!(
        "Initializing Android platform integration (arch={})",
        std::env::consts::ARCH
    );
    Ok(())
}

/// Reads device information, caching the result after the first JNI call.
///
/// Called before the activity exists the fields stay empty, so callers must
/// tolerate that (the very first frame is the only one that can hit it).
#[must_use]
pub fn device_info() -> AndroidInfo {
    static CACHE: OnceLock<RwLock<AndroidInfo>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(read_build_info()));
    cache.read().clone()
}

// ---------------------------------------------------------------------------
// JNI plumbing
// ---------------------------------------------------------------------------
//
// `jni` 0.22 changed three things that shape everything below:
//
//   * `JavaVM::attach_current_thread` takes a callback instead of returning an
//     `Env`, so every helper funnels through [`with_env`] / [`with_activity`].
//   * names and signatures are `JNIString` / parsed signature objects, not
//     `&str` — hence [`jni_name`] and the `*_signature` helpers.
//   * casting a `jobject` to a concrete type goes through `Env::as_cast`.

/// Wraps a Rust string as the MUTF-8 name/signature JNI expects.
#[cfg(target_os = "android")]
fn jni_name(text: &str) -> jni::strings::JNIString {
    jni::strings::JNIString::from(text)
}

/// An error from one of the JNI helpers, carrying the call that failed.
///
/// A dedicated type rather than `String` because `jni` 0.22 requires the error
/// of an `attach_current_thread` callback to be `From<jni::errors::Error>`, and
/// `String` is not.
#[cfg(target_os = "android")]
#[derive(Debug)]
struct JniError(String);

#[cfg(target_os = "android")]
impl std::fmt::Display for JniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for JniError {
    fn from(error: jni::errors::Error) -> Self {
        Self(format!("{error}"))
    }
}

#[cfg(target_os = "android")]
impl JniError {
    /// Builds a mapper that prefixes the raw jni message with `context`.
    fn at(context: &str) -> impl FnOnce(jni::errors::Error) -> Self + '_ {
        move |error| Self(format!("{context}: {error}"))
    }
}

/// Parses a JNI field signature such as `"I"` or `"Ljava/lang/String;"`.
#[cfg(target_os = "android")]
fn field_signature(text: &str) -> Result<jni::signature::RuntimeFieldSignature, JniError> {
    jni::signature::RuntimeFieldSignature::from_str(text)
        .map_err(|error| JniError(format!("field signature {text:?}: {error}")))
}

/// Parses a JNI method signature such as `"()V"` or `"(Ljava/lang/String;)V"`.
#[cfg(target_os = "android")]
fn method_signature(text: &str) -> Result<jni::signature::RuntimeMethodSignature, JniError> {
    jni::signature::RuntimeMethodSignature::from_str(text)
        .map_err(|error| JniError(format!("method signature {text:?}: {error}")))
}

/// Binds `name` to a parsed field signature.
///
/// The parse result owns its buffer and the `FieldSignature` borrows from it,
/// so both have to be bound; the second binding shadows the first, and the
/// original lives on until the end of the block. This has to be a statement
/// macro for the same reason — the borrowed signature cannot outlive the local
/// it points into.
///
/// Gated because it is only ever expanded from Android code, and an unused
/// `macro_rules!` warns on every host build.
#[cfg(target_os = "android")]
macro_rules! field_sig {
    ($name:ident = $text:literal) => {
        let $name = field_signature($text)?;
        let $name = jni::signature::FieldSignature::from(&$name);
    };
}

/// Binds `name` to a parsed method signature. See [`field_sig`].
#[cfg(target_os = "android")]
macro_rules! method_sig {
    ($name:ident = $text:literal) => {
        let $name = method_signature($text)?;
        let $name = jni::signature::MethodSignature::from(&$name);
    };
}

/// The `JavaVM*` the NDK passed to `ANativeActivity_onCreate`.
///
/// Borrowed rather than created: the VM is owned by the runtime and must never
/// be destroyed from here.
#[cfg(target_os = "android")]
fn java_vm() -> Option<jni::JavaVM> {
    let app = current_app()?;
    // SAFETY: `vm_as_ptr` is the pointer the NDK gave the app and it stays
    // valid for the whole process, so wrapping it is sound.
    Some(unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) })
}

/// Runs `f` with a JNI environment, attaching the calling thread if needed.
///
/// This is the only place in the crate that knows about thread attachment,
/// which is what makes the helpers below safe to call from a worker thread.
#[cfg(target_os = "android")]
fn with_env<F, T>(f: F) -> Result<T, JniError>
where
    F: FnOnce(&mut jni::Env<'_>) -> Result<T, JniError>,
{
    let vm = java_vm().ok_or_else(|| JniError("no AndroidApp installed".to_string()))?;
    vm.attach_current_thread(f)
        .map_err(|error| JniError(format!("attach_current_thread: {error}")))
}

/// Runs `f` with a JNI environment and the activity's `jobject`.
#[cfg(target_os = "android")]
fn with_activity<F, T>(f: F) -> Result<T, JniError>
where
    F: FnOnce(&mut jni::Env<'_>, &jni::objects::JObject<'_>) -> Result<T, JniError>,
{
    let app = current_app().ok_or_else(|| JniError("no AndroidApp installed".to_string()))?;
    // SAFETY: `vm_as_ptr` is the pointer the NDK gave the app and it stays
    // valid for the whole process.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };

    // `activity_as_ptr` is an unowned *global* reference owned by the NDK, so
    // it is borrowed for the duration of the call and never deleted.
    let raw: jni::sys::jobject = app.activity_as_ptr().cast();

    vm.attach_current_thread(|env| {
        // SAFETY: `raw` is a live global reference owned by the NDK, and the
        // local borrow of `raw` outlives the `Cast` produced here.
        let activity = unsafe { env.as_cast_raw::<jni::objects::JObject>(&raw) }
            .map_err(JniError::at("activity_as_ptr"))?;
        f(env, activity.as_ref())
    })
    .map_err(|error| JniError(format!("attach_current_thread: {error}")))
}

/// Reads a Java `String` out of a `jobject` that is known to hold one.
#[cfg(target_os = "android")]
fn read_java_string(
    env: &jni::Env<'_>,
    value: &jni::objects::JObject<'_>,
) -> Result<String, JniError> {
    let text = env
        .as_cast::<jni::objects::JString>(value)
        .map_err(JniError::at("JString cast"))?;
    // `try_to_string` rather than `to_string`: `JString` implements `Display`,
    // and that impl renders any JNI failure as the literal text "<JNI Error>"
    // instead of propagating it. A device string that silently became
    // "<JNI Error>" would be worse than a hard error here.
    text.try_to_string(env)
        .map_err(JniError::at("try_to_string"))
}

/// Reads a static `String` field of `class`.
#[cfg(target_os = "android")]
fn static_string(
    env: &mut jni::Env<'_>,
    class: &jni::objects::JClass<'_>,
    name: &str,
) -> Result<String, JniError> {
    field_sig!(sig = "Ljava/lang/String;");
    let value = env
        .get_static_field(class, jni_name(name), sig)
        .map_err(JniError::at(name))?
        .l()
        .map_err(JniError::at(name))?;
    read_java_string(env, &value)
}

// ---------------------------------------------------------------------------
// Device information
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
fn read_build_info() -> AndroidInfo {
    use jni::objects::{JObjectArray, JString};

    let arch = std::env::consts::ARCH;

    let result = with_env(|env| {
        let build = env.find_class(jni_name("android/os/Build"))?;

        field_sig!(int_sig = "I");
        let api_level = env
            .get_static_field(&build, jni_name("SDK_INT"), int_sig)
            .map_err(JniError::at("Build.VERSION.SDK_INT"))?
            .i()
            .map_err(JniError::at("Build.VERSION.SDK_INT"))?;

        let version_release = env
            .find_class(jni_name("android/os/Build$VERSION"))
            .ok()
            .map_or_else(String::new, |class| {
                static_string(env, &class, "RELEASE").unwrap_or_default()
            });

        // `Build.SUPPORTED_ABIS` is a `String[]`, but JNI only knows the field
        // is a plain object until it is cast to the array type.
        field_sig!(abis_sig = "[Ljava/lang/String;");
        let supported_abis = env
            .get_static_field(&build, jni_name("SUPPORTED_ABIS"), abis_sig)
            .ok()
            .and_then(|field| field.l().ok())
            .and_then(|object| {
                let cast = env.as_cast::<JObjectArray<JString>>(&object).ok()?;
                let array: &JObjectArray<JString> = &cast;
                let len = array.len(env).ok()?;
                let mut abis = Vec::with_capacity(len);
                for index in 0..len {
                    let Ok(value) = array.get_element(env, index) else {
                        continue;
                    };
                    if let Ok(value) = read_java_string(env, &value) {
                        abis.push(value);
                    }
                }
                Some(abis.join(","))
            })
            .unwrap_or_else(|| arch.to_string());

        Ok(AndroidInfo {
            api_level,
            version_release,
            display: static_string(env, &build, "DISPLAY").unwrap_or_default(),
            manufacturer: static_string(env, &build, "MANUFACTURER").unwrap_or_default(),
            model: static_string(env, &build, "MODEL").unwrap_or_default(),
            supported_abis,
            compiled_arch: arch,
        })
    });

    match result {
        Ok(info) => info,
        Err(error) => {
            tracing::warn!(%error, "could not read android.os.Build");
            AndroidInfo {
                compiled_arch: arch,
                ..AndroidInfo::default()
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
fn read_build_info() -> AndroidInfo {
    AndroidInfo {
        compiled_arch: std::env::consts::ARCH,
        ..AndroidInfo::default()
    }
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

/// Material 3 defaults shared by every platform.
#[must_use]
pub fn material3_theme() -> PlatformTheme {
    PlatformTheme {
        name: "Material 3".to_string(),
        colors: ui::platform::PlatformColors {
            primary: "#6750A4".to_string(),
            on_primary: "#FFFFFF".to_string(),
            background: "#FFFBFE".to_string(),
            on_background: "#1C1B1F".to_string(),
            surface: "#FFFBFE".to_string(),
            on_surface: "#1C1B1F".to_string(),
            error: "#BA1A1A".to_string(),
            on_error: "#FFFFFF".to_string(),
        },
        fonts: ui::platform::PlatformFonts {
            family: "Roboto".to_string(),
            size_scale: 1.0,
            weight_regular: 400,
            weight_medium: 500,
            weight_bold: 700,
        },
        spacing: ui::platform::PlatformSpacing {
            base: 4.0,
            scale: 1.0,
        },
        shapes: ui::platform::PlatformShapes {
            corner_radius_small: 4.0,
            corner_radius_medium: 8.0,
            corner_radius_large: 16.0,
        },
        motion: ui::platform::PlatformMotion {
            duration_short: 150,
            duration_medium: 250,
            duration_long: 400,
            easing_standard: "cubic-bezier(0.2, 0.0, 0.0, 1.0)".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// PlatformUi
// ---------------------------------------------------------------------------

/// [`PlatformUi`] implementation backed by the Android runtime.
#[derive(Debug, Default)]
pub struct AndroidPlatformUi;

impl AndroidPlatformUi {
    /// Creates a new adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PlatformUi for AndroidPlatformUi {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn create_window(&self, config: WindowConfig) -> AppResult<Box<dyn Window>> {
        Ok(Box::new(AndroidWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        material3_theme()
    }

    fn on_resume(&self) {
        tracing::debug!("Android activity resumed");
    }

    fn on_pause(&self) {
        tracing::debug!("Android activity paused");
    }

    fn on_destroy(&self) {
        tracing::debug!("Android activity destroyed");
    }

    fn announce_for_accessibility(&self, text: &str) {
        if let Err(error) = announce_via_jni(text) {
            tracing::warn!(%error, "could not announce accessibility text");
        }
    }

    fn set_accessibility_focus(&self, element_id: &str) {
        tracing::debug!(element_id, "accessibility focus requested");
    }
}

// ---------------------------------------------------------------------------
// Activity control
// ---------------------------------------------------------------------------

/// Asks TalkBack to read `text` through the activity's decor view.
#[cfg(target_os = "android")]
fn announce_via_jni(text: &str) -> Result<(), String> {
    use jni::objects::JValue;

    with_activity(|env, activity| {
        method_sig!(window_sig = "()Landroid/view/Window;");
        let window = env
            .call_method(activity, jni_name("getWindow"), window_sig, &[])
            .map_err(JniError::at("Activity.getWindow"))?
            .l()
            .map_err(JniError::at("Activity.getWindow"))?;

        method_sig!(decor_sig = "()Landroid/view/View;");
        let decor = env
            .call_method(&window, jni_name("getDecorView"), decor_sig, &[])
            .map_err(JniError::at("Window.getDecorView"))?
            .l()
            .map_err(JniError::at("Window.getDecorView"))?;

        method_sig!(announce_sig = "(Ljava/lang/CharSequence;)V");
        let message = env.new_string(text).map_err(JniError::at("new_string"))?;
        env.call_method(
            &decor,
            jni_name("announceForAccessibility"),
            announce_sig,
            &[JValue::Object(&message)],
        )
        .map_err(JniError::at("View.announceForAccessibility"))?
        .v()
        .map_err(JniError::at("View.announceForAccessibility"))
    })
    .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "android"))]
fn announce_via_jni(text: &str) -> Result<(), String> {
    Err(format!(
        "accessibility announcements need Android, got: {text}"
    ))
}

/// Asks Android to destroy the current activity, which ends the process.
#[cfg(target_os = "android")]
fn finish_activity() -> Result<(), String> {
    with_activity(|env, activity| {
        method_sig!(finish_sig = "()V");
        env.call_method(activity, jni_name("finish"), finish_sig, &[])
            .map_err(JniError::at("Activity.finish"))?
            .v()
            .map_err(JniError::at("Activity.finish"))
    })
    .map_err(|error| error.to_string())
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

/// Window record for the single full-screen Android surface.
///
/// Android apps own exactly one surface, so size/position requests are clamped
/// and reported back rather than forwarded to the window manager.
#[derive(Debug)]
pub struct AndroidWindow {
    title: RwLock<String>,
    size: RwLock<(u32, u32)>,
    min_size: RwLock<(u32, u32)>,
    fullscreen: RwLock<bool>,
    visible: RwLock<bool>,
}

impl AndroidWindow {
    /// Creates a window record from `config`.
    #[must_use]
    pub fn new(config: WindowConfig) -> Self {
        let min = (
            config.min_width.unwrap_or(0),
            config.min_height.unwrap_or(0),
        );
        Self {
            title: RwLock::new(config.title),
            size: RwLock::new((config.width.max(min.0), config.height.max(min.1))),
            min_size: RwLock::new(min),
            fullscreen: RwLock::new(config.fullscreen),
            visible: RwLock::new(false),
        }
    }

    /// The title last passed to [`Window::set_title`].
    #[must_use]
    pub fn title(&self) -> String {
        self.title.read().clone()
    }

    /// The size last passed to [`Window::set_size`].
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        *self.size.read()
    }
}

impl Window for AndroidWindow {
    fn show(&self) {
        *self.visible.write() = true;
    }

    fn hide(&self) {
        *self.visible.write() = false;
    }

    fn close(&self) {
        *self.visible.write() = false;
        #[cfg(target_os = "android")]
        if let Err(error) = finish_activity() {
            tracing::warn!(%error, "could not finish the activity");
        }
    }

    fn set_title(&self, title: &str) {
        *self.title.write() = title.to_string();
    }

    fn set_size(&self, width: u32, height: u32) {
        let (min_w, min_h) = *self.min_size.read();
        *self.size.write() = (width.max(min_w), height.max(min_h));
    }

    fn set_min_size(&self, width: u32, height: u32) {
        *self.min_size.write() = (width, height);
    }

    fn set_max_size(&self, width: u32, height: u32) {
        let (min_w, min_h) = *self.min_size.read();
        debug_assert!(
            width >= min_w && height >= min_h,
            "max size must not be smaller than min size"
        );
    }

    fn set_fullscreen(&self, fullscreen: bool) {
        *self.fullscreen.write() = fullscreen;
    }

    fn set_maximized(&self, _maximized: bool) {
        // The Android surface always covers the whole display.
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Android owns window placement.
    }

    fn is_visible(&self) -> bool {
        *self.visible.read()
    }

    fn is_fullscreen(&self) -> bool {
        *self.fullscreen.read()
    }

    fn is_maximized(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_info_is_readable_off_device() {
        let info = device_info();
        assert_eq!(info.compiled_arch, std::env::consts::ARCH);
        assert_eq!(info.is_android_10_or_newer(), info.api_level >= 29);
    }

    #[test]
    fn platform_ui_reports_android_and_material3() {
        let platform_ui = AndroidPlatformUi::new();
        assert_eq!(platform_ui.platform(), Platform::Android);
        let theme = platform_ui.theme();
        assert_eq!(theme.name, "Material 3");
        assert_eq!(theme.spacing.base, 4.0);
    }

    #[test]
    fn window_clamps_to_minimum_size() {
        let config = WindowConfig {
            min_width: Some(400),
            min_height: Some(300),
            ..WindowConfig::default()
        };
        let window = AndroidWindow::new(config);

        // The default size is 800x600, which is already above the minimum, so
        // construction leaves it alone: the minimum is a floor, not a target.
        assert_eq!(window.size(), (800, 600));

        // Going below the minimum is clamped up to it.
        window.set_size(100, 100);
        assert_eq!(window.size(), (400, 300));

        window.set_size(1280, 720);
        assert_eq!(window.size(), (1280, 720));
        assert!(window.is_maximized());
    }

    #[test]
    fn window_tracks_title_and_visibility() {
        let window = AndroidWindow::new(WindowConfig::default());
        assert!(!window.is_visible());
        window.show();
        assert!(window.is_visible());
        window.set_title("Rust App");
        assert_eq!(window.title(), "Rust App");
        window.hide();
        assert!(!window.is_visible());
    }

    #[test]
    fn no_app_is_installed_in_unit_tests() {
        assert!(current_app().is_none());
    }

    #[test]
    fn announcements_report_why_they_failed_off_device() {
        assert!(announce_via_jni("hello").is_err());
    }
}

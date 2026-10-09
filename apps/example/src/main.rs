//! Desktop entry point.
//!
//! The whole application lives in the library so that Android can load the exact
//! same code from `libexample_app.so` and the browser can call the
//! `#[wasm_bindgen(start)]` hook; this file only has to start it on a desktop.

/// Not compiled for wasm.
///
/// A wasm module has no `main` to run — the browser calls
/// [`example_app::run_web`] instead. Cargo still builds this target when asked
/// for `wasm32-unknown-unknown`, so the stub keeps that command working instead
/// of failing on a missing symbol.
#[cfg(target_arch = "wasm32")]
fn main() {
    unreachable!("a wasm module has no main; use run_web()");
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;

    match example_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

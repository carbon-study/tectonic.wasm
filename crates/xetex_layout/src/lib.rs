// Copyright 2020-2021 the Tectonic Project
// Licensed under the MIT License.

//! This crate provides font loading and layout code, as well as C bindings to it.
//!
//! [Tectonic]: https://tectonic-typesetting.github.io/

pub mod engine;
pub mod font;
pub mod manager;
mod utils;

mod c_api;

#[cfg(target_family = "wasm")]
pub use manager::FontRegistration;

/// Register a font from the browser host's content manifest.
///
/// The font bytes must be available at `registration.filename` through the
/// active `IoProvider` whenever XeTeX runs.
#[cfg(target_family = "wasm")]
pub fn register_font(registration: FontRegistration) -> Result<(), std::ffi::NulError> {
    manager::FontManager::register_wasm_font(registration)
}

/// Remove all manifest-backed font registrations.
///
/// This also resets the font manager so that a later registration set is used
/// by the next XeTeX run.
#[cfg(target_family = "wasm")]
pub fn clear_font_registrations() {
    manager::FontManager::clear_wasm_fonts();
}

/// Does our resulting executable link correctly?
#[test]
fn linkage() {}

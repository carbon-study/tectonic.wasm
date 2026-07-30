use crate::c_api::{Fixed, PlatformFontRef, RawPlatformFontRef};
use std::ptr::NonNull;

pub fn fix_to_d(f: Fixed) -> f64 {
    f as f64 / 65536.0
}

pub fn d_to_fix(d: f64) -> Fixed {
    (d * 65536.0 + 0.5) as Fixed
}

pub fn raw_to_rs(font: RawPlatformFontRef) -> Option<PlatformFontRef> {
    #[cfg(target_os = "macos")]
    let out = {
        use tectonic_mac_core::CoreType;
        // SAFETY: Pointer must be from us, and is thus a borrowed ref
        NonNull::new(font.cast_mut()).map(|ptr| unsafe { PlatformFontRef::new_borrowed(ptr) })
    };
    #[cfg(not(any(target_os = "macos", target_family = "wasm")))]
    // SAFETY: Pointer must be from us, and is thus a borrowed ref
    let out = { unsafe { NonNull::new(font).map(|p| PlatformFontRef::from_raw_borrowed(p)) } };
    #[cfg(target_family = "wasm")]
    let out = {
        let ptr = NonNull::new(font.cast::<crate::manager::wasm::WasmFontDescriptor>());
        ptr.map(|ptr| {
            // SAFETY: WASM platform-font pointers are created from an `Arc`
            // retained by the font manager. Incrementing first gives the
            // returned wrapper its own strong reference.
            unsafe {
                std::sync::Arc::increment_strong_count(ptr.as_ptr());
                std::sync::Arc::from_raw(ptr.as_ptr())
            }
        })
    };
    out
}

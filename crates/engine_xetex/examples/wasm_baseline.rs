//! Temporary size baseline for the complete XeTeX engine.

use tectonic_bridge_core::CoreBridgeState;
use tectonic_engine_xetex::c_api;

/// Invoke the complete XeTeX C engine through its real entry point.
///
/// # Safety
///
/// All pointers must satisfy the requirements of
/// [`c_api::tt_engine_xetex_main`].
#[no_mangle]
pub unsafe extern "C" fn tectonic_wasm_baseline_xetex_main(
    api: *mut CoreBridgeState<'_>,
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: u64,
) -> libc::c_int {
    let Some(api) = api.as_mut() else {
        return -1;
    };

    c_api::tt_engine_xetex_main(api, dump_name, input_file_name, build_date)
}

/// Return the first Unicode line-break boundary in a UTF-32 buffer.
///
/// This temporary export keeps libgrapheme's UAX #14 implementation and
/// property tables alive for linked-size measurements.
///
/// # Safety
///
/// `text` must point to `length` initialized `u32` code points.
#[cfg(feature = "libgrapheme")]
#[no_mangle]
pub unsafe extern "C" fn tectonic_wasm_baseline_next_line_break(
    text: *const u32,
    length: usize,
) -> usize {
    let text = if length == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees that this buffer is readable.
        unsafe { std::slice::from_raw_parts(text, length) }
    };

    tectonic_bridge_grapheme::next_line_break(text)
}

/// Exercise libgrapheme's full UAX #9 pipeline and return its output length.
///
/// # Safety
///
/// `text` must point to `length` initialized `u32` code points.
#[cfg(feature = "libgrapheme")]
#[no_mangle]
pub unsafe extern "C" fn tectonic_wasm_baseline_bidi_probe(
    text: *const u32,
    length: usize,
) -> usize {
    let text = if length == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees that this buffer is readable.
        unsafe { std::slice::from_raw_parts(text, length) }
    };
    let (state, _) = tectonic_bridge_grapheme::preprocess_bidi_paragraph(
        text,
        tectonic_bridge_grapheme::BidiDirection::Neutral,
    );
    let levels = tectonic_bridge_grapheme::bidi_embedding_levels(&state);
    let reordered = tectonic_bridge_grapheme::reorder_bidi_line(text, &state);

    levels.len() ^ reordered.len()
}

fn main() {
    std::hint::black_box(tectonic_wasm_baseline_xetex_main);

    #[cfg(feature = "libgrapheme")]
    std::hint::black_box(tectonic_wasm_baseline_next_line_break);
    #[cfg(feature = "libgrapheme")]
    std::hint::black_box(tectonic_wasm_baseline_bidi_probe);
}

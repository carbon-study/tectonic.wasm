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

fn main() {
    std::hint::black_box(tectonic_wasm_baseline_xetex_main);
}

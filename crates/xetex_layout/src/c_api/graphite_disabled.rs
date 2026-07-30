//! Stable C ABI stubs for builds without Graphite.

use crate::c_api::XeTeXLayoutEngine;
use tectonic_bridge_harfbuzz as hb;

#[no_mangle]
pub unsafe extern "C" fn countGraphiteFeatures(_engine: XeTeXLayoutEngine) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureCode(_engine: XeTeXLayoutEngine, _index: u32) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn countGraphiteFeatureSettings(
    _engine: XeTeXLayoutEngine,
    _feature_id: u32,
) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureSettingCode(
    _engine: XeTeXLayoutEngine,
    _feature_id: u32,
    _index: u32,
) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureDefaultSetting(
    _engine: XeTeXLayoutEngine,
    _feature_id: u32,
) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureLabel(
    _engine: XeTeXLayoutEngine,
    _feature_id: u32,
) -> *const libc::c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureSettingLabel(
    _engine: XeTeXLayoutEngine,
    _feature_id: u32,
    _setting_id: u32,
) -> *const libc::c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeature(
    _engine: XeTeXLayoutEngine,
    _start: *const libc::c_char,
    _end: *const libc::c_char,
    _feature: *mut hb::Tag,
    _value: *mut libc::c_int,
) -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeatureNamed(
    _engine: XeTeXLayoutEngine,
    _name: *const libc::c_char,
    _name_length: libc::c_int,
) -> libc::c_long {
    -1
}

#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeatureSettingNamed(
    _engine: XeTeXLayoutEngine,
    _id: u32,
    _name: *const libc::c_char,
    _name_length: libc::c_int,
) -> libc::c_long {
    -1
}

#[no_mangle]
pub unsafe extern "C" fn initGraphiteBreaking(
    _engine: XeTeXLayoutEngine,
    _text: *const u16,
    _text_length: libc::c_uint,
) -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn findNextGraphiteBreak(_engine: XeTeXLayoutEngine) -> libc::c_int {
    -1
}

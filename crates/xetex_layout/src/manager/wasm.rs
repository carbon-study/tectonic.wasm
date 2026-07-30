//! Manifest-backed font discovery for WebAssembly.

use super::{FontInfo, FontManagerBackend, FontMaps, FontRegistration, NameCollection};
use crate::c_api::PlatformFontRef;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::{CStr, CString, NulError};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

thread_local! {
    static REGISTERED_FONTS: RefCell<Vec<PlatformFontRef>> = const { RefCell::new(Vec::new()) };
}

pub struct WasmBackend;

/// A font-file identity and its manifest-provided naming metadata.
#[derive(Debug)]
pub struct WasmFontDescriptor {
    filename: CString,
    index: usize,
    names: NameCollection,
    weight: u16,
    width: u16,
    slant: i16,
    is_regular: bool,
    is_bold: bool,
    is_italic: bool,
}

impl WasmFontDescriptor {
    pub(crate) fn filename(&self) -> &CStr {
        &self.filename
    }

    pub(crate) fn index(&self) -> usize {
        self.index
    }
}

impl PartialEq for WasmFontDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.filename == other.filename && self.index == other.index
    }
}

impl Eq for WasmFontDescriptor {}

impl Hash for WasmFontDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.filename.hash(state);
        self.index.hash(state);
    }
}

fn strings_to_cstrings(strings: Vec<String>) -> Result<Vec<CString>, NulError> {
    strings.into_iter().map(CString::new).collect()
}

pub(crate) fn register_font(registration: FontRegistration) -> Result<PlatformFontRef, NulError> {
    let postscript_name = CString::new(registration.postscript_name)?;
    let mut family_names = strings_to_cstrings(registration.family_names)?;
    if family_names.is_empty() {
        family_names.push(postscript_name.clone());
    }
    let mut style_names = strings_to_cstrings(registration.style_names)?;
    if style_names.is_empty() {
        style_names.push(CString::default());
    }

    let descriptor = Arc::new(WasmFontDescriptor {
        filename: CString::new(registration.filename)?,
        index: registration.index as usize,
        names: NameCollection {
            family_names,
            style_names,
            full_names: strings_to_cstrings(registration.full_names)?,
            ps_name: Some(postscript_name),
        },
        weight: registration.weight,
        width: registration.width,
        slant: registration.slant,
        is_regular: registration.is_regular,
        is_bold: registration.is_bold,
        is_italic: registration.is_italic,
    });

    Ok(REGISTERED_FONTS.with_borrow_mut(|fonts| {
        if let Some(existing) = fonts
            .iter()
            .find(|font| font.filename == descriptor.filename && font.index == descriptor.index)
        {
            return existing.clone();
        }
        fonts.push(descriptor.clone());
        descriptor
    }))
}

pub(crate) fn registered_fonts() -> Vec<PlatformFontRef> {
    REGISTERED_FONTS.with_borrow(Clone::clone)
}

pub(crate) fn clear_registered_fonts() {
    REGISTERED_FONTS.with_borrow_mut(Vec::clear);
}

impl FontManagerBackend for WasmBackend {
    fn get_platform_font_desc<'a>(&'a self, font: &'a PlatformFontRef) -> Cow<'a, CStr> {
        Cow::Borrowed(font.filename())
    }

    fn get_op_size_rec_and_style_flags(&self, font: &mut FontInfo) {
        font.weight = font.font_ref.weight;
        font.width = font.font_ref.width;
        font.slant = font.font_ref.slant;
        font.is_reg = font.font_ref.is_regular;
        font.is_bold = font.font_ref.is_bold;
        font.is_italic = font.font_ref.is_italic;
    }

    fn search_for_host_platform_fonts(&mut self, _maps: &mut FontMaps, _name: &CStr) {}

    fn read_names(&self, font: PlatformFontRef) -> NameCollection {
        font.names.clone()
    }
}

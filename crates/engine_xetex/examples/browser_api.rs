//! Browser-facing XeTeX entry point with asynchronous host resource loading.
//!
//! The generated Emscripten module is driven through `tectonic_compile`.
//! Callers supply the primary TeX source and manifest-derived font metadata;
//! the result remains owned by Rust until the next invocation so that the
//! JavaScript facade can copy out the XDV and structured diagnostics.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fmt::Arguments,
    io::{self, Cursor, Write},
    ptr, slice, str,
    sync::{Arc, Mutex},
};
use tectonic_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use tectonic_engine_xetex::{TexEngine, TexOutcome};
use tectonic_errors::Error;
use tectonic_io_base::{InputHandle, InputOrigin, IoProvider, OpenResult, OutputHandle};
use tectonic_status_base::{MessageKind, StatusBackend};
#[cfg(target_family = "wasm")]
use tectonic_xetex_layout::FontRegistration;

#[cfg(target_os = "emscripten")]
unsafe extern "C" {
    fn tectonic_asyncify_load(
        name: *const libc::c_char,
        is_format: libc::c_int,
        data_out: *mut *mut u8,
        len_out: *mut usize,
    ) -> libc::c_int;
}

#[derive(Clone, Default)]
struct SharedOutputs(Arc<Mutex<HashMap<String, Vec<u8>>>>);

struct SharedWriter {
    name: String,
    outputs: SharedOutputs,
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.outputs
            .0
            .lock()
            .unwrap()
            .get_mut(&self.name)
            .expect("output initialized before writing")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BrowserIo {
    primary_name: String,
    primary_bytes: Vec<u8>,
    outputs: SharedOutputs,
}

impl BrowserIo {
    fn load(&mut self, name: &str, is_format: bool) -> OpenResult<InputHandle> {
        let c_name = match CString::new(name) {
            Ok(value) => value,
            Err(error) => return OpenResult::Err(error.into()),
        };
        let mut data = ptr::null_mut();
        let mut len = 0usize;

        #[cfg(target_os = "emscripten")]
        let result = unsafe {
            tectonic_asyncify_load(c_name.as_ptr(), is_format.into(), &mut data, &mut len)
        };

        #[cfg(not(target_os = "emscripten"))]
        let result = {
            let _ = (c_name, is_format, &mut data, &mut len);
            -1
        };

        match result {
            0 => OpenResult::NotAvailable,
            1 => {
                if data.is_null() && len != 0 {
                    return OpenResult::Err(
                        io::Error::other("async loader returned a null buffer").into(),
                    );
                }

                let bytes = if len == 0 {
                    Vec::new()
                } else {
                    // The host bridge allocates with Emscripten's malloc. Copy
                    // into Rust-owned storage before entering the I/O layer.
                    let bytes = unsafe { slice::from_raw_parts(data, len) }.to_vec();
                    unsafe { libc::free(data.cast()) };
                    bytes
                };

                OpenResult::Ok(InputHandle::new_read_only(
                    name,
                    Cursor::new(bytes),
                    InputOrigin::Other,
                ))
            }
            _ => OpenResult::Err(
                io::Error::other(format!("async host load failed for {name}")).into(),
            ),
        }
    }
}

impl IoProvider for BrowserIo {
    fn output_open_name(&mut self, name: &str) -> OpenResult<OutputHandle> {
        self.outputs
            .0
            .lock()
            .unwrap()
            .insert(name.to_owned(), Vec::new());
        OpenResult::Ok(OutputHandle::new(
            name,
            SharedWriter {
                name: name.to_owned(),
                outputs: self.outputs.clone(),
            },
        ))
    }

    fn output_open_stdout(&mut self) -> OpenResult<OutputHandle> {
        OpenResult::Ok(OutputHandle::new("", io::sink()))
    }

    fn input_open_name(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        if let Some(bytes) = self.outputs.0.lock().unwrap().get(name).cloned() {
            return OpenResult::Ok(InputHandle::new(
                name,
                Cursor::new(bytes),
                InputOrigin::Other,
            ));
        }
        self.load(name, false)
    }

    fn input_open_primary(&mut self, _status: &mut dyn StatusBackend) -> OpenResult<InputHandle> {
        OpenResult::Ok(InputHandle::new_read_only(
            &self.primary_name,
            Cursor::new(self.primary_bytes.clone()),
            InputOrigin::Other,
        ))
    }

    fn input_open_format(
        &mut self,
        name: &str,
        _status: &mut dyn StatusBackend,
    ) -> OpenResult<InputHandle> {
        self.load(name, true)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserFontRegistration {
    filename: String,
    index: u32,
    postscript_name: String,
    family_names: Vec<String>,
    style_names: Vec<String>,
    full_names: Vec<String>,
    weight: u16,
    width: u16,
    slant: i16,
    is_regular: bool,
    is_bold: bool,
    is_italic: bool,
}

#[cfg(target_family = "wasm")]
impl From<BrowserFontRegistration> for FontRegistration {
    fn from(value: BrowserFontRegistration) -> Self {
        FontRegistration {
            filename: value.filename,
            index: value.index,
            postscript_name: value.postscript_name,
            family_names: value.family_names,
            style_names: value.style_names,
            full_names: value.full_names,
            weight: value.weight,
            width: value.width,
            slant: value.slant,
            is_regular: value.is_regular,
            is_bold: value.is_bold,
            is_italic: value.is_italic,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    kind: &'static str,
    message: String,
    causes: Vec<String>,
}

#[derive(Default)]
struct DiagnosticStatus {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticStatus {
    fn push_error(&mut self, message: impl Into<String>, error: &Error) {
        self.diagnostics.push(Diagnostic {
            kind: "error",
            message: message.into(),
            causes: error.chain().map(ToString::to_string).collect(),
        });
    }
}

impl StatusBackend for DiagnosticStatus {
    fn report(&mut self, kind: MessageKind, args: Arguments, error: Option<&Error>) {
        self.diagnostics.push(Diagnostic {
            kind: match kind {
                MessageKind::Note => "note",
                MessageKind::Warning => "warning",
                MessageKind::Error => "error",
            },
            message: args.to_string(),
            causes: error
                .map(|value| value.chain().map(ToString::to_string).collect())
                .unwrap_or_default(),
        });
    }

    fn report_error(&mut self, error: &Error) {
        self.push_error("engine error", error);
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        self.diagnostics.push(Diagnostic {
            kind: "error-log",
            message: String::from_utf8_lossy(output).into_owned(),
            causes: Vec::new(),
        });
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultMetadata {
    ok: bool,
    outcome: Option<&'static str>,
    output_name: Option<String>,
    diagnostics: Vec<Diagnostic>,
    error: Option<String>,
}

struct CompileResult {
    metadata_json: CString,
    xdv: Vec<u8>,
}

impl CompileResult {
    fn from_parts(metadata: ResultMetadata, xdv: Vec<u8>) -> Self {
        let json = serde_json::to_string(&metadata).unwrap_or_else(|error| {
            format!(
                r#"{{"ok":false,"outcome":null,"outputName":null,"diagnostics":[],"error":"failed to encode result: {error}"}}"#
            )
        });
        Self {
            metadata_json: CString::new(json).expect("JSON never contains literal NUL bytes"),
            xdv,
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self::from_parts(
            ResultMetadata {
                ok: false,
                outcome: None,
                output_name: None,
                diagnostics: Vec::new(),
                error: Some(message.into()),
            },
            Vec::new(),
        )
    }
}

static LAST_RESULT: Mutex<Option<CompileResult>> = Mutex::new(None);

fn compile(
    primary_bytes: Vec<u8>,
    primary_name: String,
    format_name: String,
    fonts: Vec<BrowserFontRegistration>,
) -> CompileResult {
    #[cfg(target_family = "wasm")]
    {
        tectonic_xetex_layout::clear_font_registrations();
        for font in fonts {
            if let Err(error) = tectonic_xetex_layout::register_font(font.into()) {
                return CompileResult::failure(format!(
                    "invalid font registration contains a NUL byte: {error}"
                ));
            }
        }
    }
    #[cfg(not(target_family = "wasm"))]
    let _ = fonts;

    let outputs = SharedOutputs::default();
    let io = BrowserIo {
        primary_name: primary_name.clone(),
        primary_bytes,
        outputs: outputs.clone(),
    };
    let mut driver = MinimalDriver::new(io);
    let mut status = DiagnosticStatus::default();
    let mut launcher = CoreBridgeLauncher::new(&mut driver, &mut status);
    let engine_result = TexEngine::default().process(&mut launcher, &format_name, &primary_name);

    let (outcome, engine_error) = match engine_result {
        Ok(TexOutcome::Spotless) => (Some("spotless"), None),
        Ok(TexOutcome::Warnings) => (Some("warnings"), None),
        Ok(TexOutcome::Errors) => (Some("errors"), None),
        Err(error) => {
            status.push_error("XeTeX compilation failed", &error);
            (None, Some(error.to_string()))
        }
    };

    let mut output_name = None;
    let mut xdv = Vec::new();
    for (name, bytes) in outputs.0.lock().unwrap().iter() {
        if name.ends_with(".xdv") {
            output_name = Some(name.clone());
            xdv = bytes.clone();
            break;
        }
    }
    let ok = engine_error.is_none() && outcome != Some("errors") && !xdv.is_empty();
    let error = if engine_error.is_some() {
        engine_error
    } else if outcome == Some("errors") {
        Some("XeTeX completed with errors".to_owned())
    } else if xdv.is_empty() {
        Some("XeTeX did not produce XDV output".to_owned())
    } else {
        None
    };

    CompileResult::from_parts(
        ResultMetadata {
            ok,
            outcome,
            output_name,
            diagnostics: status.diagnostics,
            error,
        },
        xdv,
    )
}

unsafe fn copy_bytes(pointer: *const u8, length: usize, label: &str) -> Result<Vec<u8>, String> {
    if pointer.is_null() {
        return if length == 0 {
            Ok(Vec::new())
        } else {
            Err(format!("{label} pointer is null for {length} bytes"))
        };
    }
    Ok(slice::from_raw_parts(pointer, length).to_vec())
}

unsafe fn copy_c_string(pointer: *const libc::c_char, label: &str) -> Result<String, String> {
    if pointer.is_null() {
        return Err(format!("{label} pointer is null"));
    }
    CStr::from_ptr(pointer)
        .to_str()
        .map(str::to_owned)
        .map_err(|error| format!("{label} is not UTF-8: {error}"))
}

/// Compile one caller-provided TeX document.
///
/// The call may suspend through Asyncify. All argument memory must remain
/// valid until the returned JavaScript promise resolves.
#[no_mangle]
pub unsafe extern "C" fn tectonic_compile(
    source_pointer: *const u8,
    source_length: usize,
    input_name_pointer: *const libc::c_char,
    format_name_pointer: *const libc::c_char,
    fonts_json_pointer: *const libc::c_char,
) -> libc::c_int {
    *LAST_RESULT.lock().unwrap() = None;

    let arguments = (|| {
        let source = copy_bytes(source_pointer, source_length, "source")?;
        let input_name = copy_c_string(input_name_pointer, "input name")?;
        let format_name = copy_c_string(format_name_pointer, "format name")?;
        let fonts_json = copy_c_string(fonts_json_pointer, "font metadata")?;
        if input_name.is_empty() {
            return Err("input name must not be empty".to_owned());
        }
        if format_name.is_empty() {
            return Err("format name must not be empty".to_owned());
        }
        let fonts = serde_json::from_str(&fonts_json)
            .map_err(|error| format!("invalid font metadata: {error}"))?;
        Ok((source, input_name, format_name, fonts))
    })();

    let result = match arguments {
        Ok((source, input_name, format_name, fonts)) => {
            compile(source, input_name, format_name, fonts)
        }
        Err(error) => CompileResult::failure(error),
    };
    *LAST_RESULT.lock().unwrap() = Some(result);
    0
}

#[no_mangle]
/// Return the latest compile result as a NUL-terminated JSON object.
///
/// The pointer remains valid until the next call to [`tectonic_compile`].
pub extern "C" fn tectonic_result_json() -> *const libc::c_char {
    LAST_RESULT
        .lock()
        .unwrap()
        .as_ref()
        .map(|result| result.metadata_json.as_ptr())
        .unwrap_or(ptr::null())
}

#[no_mangle]
/// Return a pointer to the latest compile's XDV bytes.
///
/// The pointer remains valid until the next call to [`tectonic_compile`].
pub extern "C" fn tectonic_result_xdv_pointer() -> *const u8 {
    LAST_RESULT
        .lock()
        .unwrap()
        .as_ref()
        .map(|result| result.xdv.as_ptr())
        .unwrap_or(ptr::null())
}

#[no_mangle]
/// Return the number of bytes available from [`tectonic_result_xdv_pointer`].
pub extern "C" fn tectonic_result_xdv_length() -> usize {
    LAST_RESULT
        .lock()
        .unwrap()
        .as_ref()
        .map(|result| result.xdv.len())
        .unwrap_or(0)
}

fn main() {}

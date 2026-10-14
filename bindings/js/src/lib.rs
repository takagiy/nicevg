//! A C interface to nicevg for runtimes that load native libraries, such as
//! Bun's FFI or koffi on Node: a diagram as JSON in, the fixed and arranged
//! diagram as JSON out.
//!
//! The request is either `{"svg": "..."}` or a sketch (`{"nodes": [...],
//! "connectors": [...]}`). The reply is `{"ok": ...}`, holding the fix
//! result, or `{"error": "..."}`.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Value, json};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

enum Input {
    Svg(String),
    Sketch(nicevg::Sketch),
}

fn read(request: &[u8]) -> Result<Input, String> {
    let value: Value = serde_json::from_slice(request).map_err(|error| format!("invalid JSON: {error}"))?;
    if let Some(svg) = value.get("svg") {
        return svg
            .as_str()
            .map(|svg| Input::Svg(svg.to_owned()))
            .ok_or_else(|| "svg: expected a string".to_owned());
    }
    serde_path_to_error::deserialize(value)
        .map(Input::Sketch)
        .map_err(|error| format!("{}: {}", error.path(), error.inner()))
}

fn render(request: &[u8]) -> Result<Value, String> {
    let result = match read(request)? {
        Input::Svg(svg) => nicevg::arrange(&svg).map_err(|error| error.to_string())?,
        Input::Sketch(sketch) => nicevg::arrange_sketch(&sketch).map_err(|error| error.to_string())?,
    };
    Ok(serde_json::to_value(result).expect("serializable result"))
}

/// Fixes and arranges the diagram in the JSON request of `length` bytes at
/// `request`, and returns the JSON reply, its length written to
/// `reply_length`. The caller frees the reply with [`nicevg_free`].
///
/// # Safety
///
/// `request` points to `length` readable bytes and `reply_length` to a
/// writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nicevg_render(request: *const u8, length: usize, reply_length: *mut usize) -> *mut u8 {
    keep_loaded();
    let request = if length == 0 {
        &[][..]
    } else {
        // SAFETY: the caller passes `length` readable bytes.
        unsafe { std::slice::from_raw_parts(request, length) }
    };
    let reply = match catch_unwind(AssertUnwindSafe(|| render(request))) {
        Ok(Ok(value)) => json!({ "ok": value }),
        Ok(Err(message)) => json!({ "error": message }),
        Err(_) => json!({ "error": "nicevg stopped on an internal error" }),
    };
    let bytes = serde_json::to_vec(&reply)
        .expect("serializable reply")
        .into_boxed_slice();
    // SAFETY: the caller passes a writable `usize`.
    unsafe { *reply_length = bytes.len() };
    Box::into_raw(bytes).cast::<u8>()
}

/// Keeps the library in memory until the process ends, once called. A
/// thread that has finished its work still runs code in the library as it
/// exits, and koffi unloads the library while Node exits: on Windows, an
/// exiting thread then crashed the process.
fn keep_loaded() {
    #[cfg(windows)]
    {
        static PINNED: std::sync::Once = std::sync::Once::new();
        PINNED.call_once(|| {
            unsafe extern "system" {
                fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut *mut std::ffi::c_void) -> i32;
            }
            const PIN: u32 = 0x1;
            const FROM_ADDRESS: u32 = 0x4;
            let mut module = std::ptr::null_mut();
            // SAFETY: the name is an address inside this library, as FROM_ADDRESS asks.
            unsafe { GetModuleHandleExW(PIN | FROM_ADDRESS, keep_loaded as *const u16, &mut module) };
        });
    }
}

/// Frees a reply [`nicevg_render`] returned.
///
/// # Safety
///
/// `reply` and `length` are a reply and its length from [`nicevg_render`],
/// freed only once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nicevg_free(reply: *mut u8, length: usize) {
    // SAFETY: the reply was a boxed slice of `length` bytes.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(reply, length)) });
}

//! Embedding and reading the `RT_RCDATA` configuration resource.

use std::ffi::c_void;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, FindResourceW, LoadLibraryExW, LoadResource,
    LockResource, SizeofResource, UpdateResourceW,
};

/// `RT_RCDATA`, the predefined resource type for raw data.
pub(super) const RT_RCDATA: u16 = 10;

/// Language id of a resource. It is not validated on lookup, so any value
/// works as long as the writer and the reader agree.
const ENGLISH_US: u16 = 0x0409;

/// Load flags that map the image as data: searching it for resources runs no
/// code, which matters when a shim for a GUI target is not meant to start.
const LOAD_LIBRARY_AS_DATAFILE: u32 = 0x2;

/// Resource lookup types that may be either a `MAKEINTRESOURCE` ordinal or a
/// pointer to a real string.
#[derive(Clone, Copy)]
enum ResourceId {
    Ordinal(u16),
    Name(*const u16),
}

impl ResourceId {
    /// Predefined types are ordinals packed into a pointer-sized integer, not
    /// pointers to a buffer holding the number. `FindResourceW` dereferences a
    /// pointer-form type as a string, so the buffer form finds nothing.
    fn ordinal(value: u16) -> Self {
        Self::Ordinal(value)
    }

    fn name(units: &[u16]) -> Self {
        Self::Name(units.as_ptr())
    }

    fn as_ptr(self) -> *const u16 {
        match self {
            Self::Ordinal(value) => value as usize as *const u16,
            Self::Name(pointer) => pointer,
        }
    }
}

/// Put `payload` into `image` as `RT_RCDATA` under `name`.
pub(super) fn write(
    image: &Path,
    name: &[u16],
    payload: &[u8],
    language: Option<u16>,
) -> Result<(), String> {
    let image = wides(&image.to_string_lossy());
    let update = unsafe { BeginUpdateResourceW(image.as_ptr(), 0) };
    if update.is_null() {
        return Err(format!(
            "BeginUpdateResourceW failed (error {})",
            last_error()
        ));
    }

    match language {
        Some(language) => update_resource(update, name, payload, language),
        // `UpdateResourceW` cannot delete a language by id, only by `u16::MAX`.
        None => update_resource(update, name, payload, u16::MAX),
    }
    .and_then(|()| {
        if unsafe { EndUpdateResourceW(update, 0) } == 0 {
            Err(format!(
                "EndUpdateResourceW failed (error {})",
                last_error()
            ))
        } else {
            Ok(())
        }
    })
    .inspect_err(|_| unsafe {
        // Discard the in-memory edits when the update cannot be committed.
        EndUpdateResourceW(update, 1);
    })
}

fn update_resource(
    update: HANDLE,
    name: &[u16],
    payload: &[u8],
    language: u16,
) -> Result<(), String> {
    let data = if payload.is_empty() {
        null()
    } else {
        payload.as_ptr() as *const c_void
    };
    let ok = unsafe {
        UpdateResourceW(
            update,
            ResourceId::ordinal(RT_RCDATA).as_ptr(),
            ResourceId::name(name).as_ptr(),
            language,
            data,
            payload.len() as u32,
        )
    };
    if ok == 0 {
        return Err(format!("UpdateResourceW failed (error {})", last_error()));
    }
    Ok(())
}

/// Read the configuration resource out of an image file.
#[allow(dead_code)]
pub(super) fn read(image: &Path, name: &[u16]) -> Result<Vec<u8>, String> {
    let image = wides(&image.to_string_lossy());
    let module = unsafe { LoadLibraryExW(image.as_ptr(), null_mut(), LOAD_LIBRARY_AS_DATAFILE) };
    if module.is_null() {
        return Err(format!("LoadLibraryExW failed (error {})", last_error()));
    }

    let result = read_from(module, name);
    unsafe {
        windows_sys::Win32::Foundation::FreeLibrary(module);
    }
    result
}

fn read_from(module: HMODULE, name: &[u16]) -> Result<Vec<u8>, String> {
    let resource = unsafe {
        FindResourceW(
            module,
            ResourceId::name(name).as_ptr(),
            ResourceId::ordinal(RT_RCDATA).as_ptr(),
        )
    };
    if resource.is_null() {
        let error = last_error();
        let code = if error == 0 {
            ERROR_FILE_NOT_FOUND
        } else {
            error
        };
        return Err(format!(
            "FindResourceW(RT_RCDATA) failed (error {code}) for the named resource"
        ));
    }

    let size = unsafe { SizeofResource(module, resource) } as usize;
    let loaded = unsafe { LoadResource(module, resource) };
    if loaded.is_null() || size == 0 {
        return Err(String::from("LoadResource failed"));
    }

    let data = unsafe { LockResource(loaded) };
    if data.is_null() {
        return Err(String::from("LockResource failed"));
    }

    Ok(unsafe { std::slice::from_raw_parts(data as *const u8, size) }.to_vec())
}

#[cfg(test)]
pub(super) fn make_int_resource_for_test(value: u16) -> *const u16 {
    ResourceId::ordinal(value).as_ptr()
}

/// Whether the resource is found when the type is passed as an ordinal.
#[cfg(test)]
pub(super) fn find_by_ordinal_for_test(image: &Path, name: &[u16], ordinal: u16) -> bool {
    find_with_type(image, name, ordinal as usize as *const u16)
}

/// Whether the resource is found when the type is passed as a pointer to a
/// buffer holding the number, which is the bug this guards against.
#[cfg(test)]
pub(super) fn find_by_buffer_for_test(image: &Path, name: &[u16], ordinal: u16) -> bool {
    let buffer = [ordinal, 0];
    find_with_type(image, name, buffer.as_ptr())
}

#[cfg(test)]
fn find_with_type(image: &Path, name: &[u16], resource_type: *const u16) -> bool {
    let image = wides(&image.to_string_lossy());
    let module = unsafe { LoadLibraryExW(image.as_ptr(), null_mut(), LOAD_LIBRARY_AS_DATAFILE) };
    if module.is_null() {
        return false;
    }
    let found =
        !unsafe { FindResourceW(module, ResourceId::name(name).as_ptr(), resource_type) }.is_null();
    unsafe {
        windows_sys::Win32::Foundation::FreeLibrary(module);
    }
    found
}

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

/// Encode `text` as a NUL-terminated UTF-16 buffer for a `PCWSTR` parameter.
fn wides(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

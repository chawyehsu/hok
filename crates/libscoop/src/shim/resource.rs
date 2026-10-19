//! Embedding the configuration and the target's icon into a shim image.

use std::ffi::c_void;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, FreeLibrary, GetLastError, HANDLE, HMODULE,
};
use windows_sys::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, FindResourceExW, FindResourceW, LoadLibraryExW,
    LoadResource, LockResource, SizeofResource, UpdateResourceW,
};

/// `RT_RCDATA`, the predefined resource type for raw data.
pub(super) const RT_RCDATA: u16 = 10;

/// `RT_ICON`, a single icon image.
pub(super) const RT_ICON: u16 = 3;

/// `RT_GROUP_ICON`, an icon directory naming a set of `RT_ICON` images.
pub(super) const RT_GROUP_ICON: u16 = 14;

/// Language id of a resource. It is not validated on lookup, so any value
/// works as long as the writer and the reader agree.
const ENGLISH_US: u16 = 0x0409;

/// `MAKELANGID(LANG_NEUTRAL, SUBLANG_NEUTRAL)`: match any language.
const LANG_NEUTRAL: u16 = 0x0000;

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

/// An icon group and the images it references, ready to be written.
pub(super) struct IconSet {
    /// Resource id of the group in the source image.
    pub(super) group_id: u16,
    /// The `RT_GROUP_ICON` payload, byte for byte.
    pub(super) group: Vec<u8>,
    /// Each referenced image, with the id the group's directory entries use.
    pub(super) images: Vec<(u16, Vec<u8>)>,
}

impl IconSet {
    /// How many images the group carries.
    pub(super) fn image_count(&self) -> usize {
        self.images.len()
    }
}

/// Put `payload` into `image` as `RT_RCDATA` under `name`.
pub(super) fn write(
    image: &Path,
    name: &[u16],
    payload: &[u8],
    language: Option<u16>,
) -> Result<(), String> {
    update(
        image,
        |update, language| {
            update_resource(update, RT_RCDATA, ResourceId::name(name), language, payload)
        },
        language,
    )
}

/// Replace `image`'s icon with the icon of `source`.
///
/// The template's own icon stays when the source has none: a package whose
/// program carries no icon is not improved by making its shim iconless.
pub(super) fn copy_icon(image: &Path, source: &Path) -> Result<(), String> {
    let Some(icon) = read_icon(source)? else {
        return Ok(());
    };

    // Windows keys icon groups by id, and a stale group left behind would still
    // be selectable, so the template's icons are removed before theirs are
    // written. The set to remove is read from the image, since deleting an id
    // that does not exist is an error rather than a no-op.
    let existing = existing_icon_ids(image)?;

    update(
        image,
        |update, language| {
            for (resource_type, id) in &existing {
                let ok = unsafe {
                    UpdateResourceW(
                        update,
                        ResourceId::ordinal(*resource_type).as_ptr(),
                        ResourceId::ordinal(*id).as_ptr(),
                        language,
                        null(),
                        0,
                    )
                };
                if ok == 0 {
                    return Err(format!(
                        "UpdateResourceW(delete type {resource_type} id {id}) failed (error {})",
                        last_error()
                    ));
                }
            }

            for (id, bytes) in &icon.images {
                update_resource(update, RT_ICON, ResourceId::ordinal(*id), language, bytes)?;
            }
            update_resource(
                update,
                RT_GROUP_ICON,
                ResourceId::ordinal(icon.group_id),
                language,
                &icon.group,
            )
        },
        Some(ENGLISH_US),
    )
}

/// Apply a set of resource edits through one `BeginUpdateResourceW` session.
///
/// All edits are committed together, so a failure part way through leaves the
/// image untouched rather than carrying half a configuration.
fn update(
    image: &Path,
    edits: impl FnOnce(HANDLE, u16) -> Result<(), String>,
    language: Option<u16>,
) -> Result<(), String> {
    let image = wides(&image.to_string_lossy());
    let handle = unsafe { BeginUpdateResourceW(image.as_ptr(), 0) };
    if handle.is_null() {
        return Err(format!(
            "BeginUpdateResourceW failed (error {})",
            last_error()
        ));
    }

    // `UpdateResourceW` cannot delete a language by id, only by `u16::MAX`.
    let language = language.unwrap_or(u16::MAX);

    edits(handle, language)
        .and_then(|()| {
            if unsafe { EndUpdateResourceW(handle, 0) } == 0 {
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
            EndUpdateResourceW(handle, 1);
        })
}

/// Every icon resource present in `image`, as `(type, id)` pairs.
fn existing_icon_ids(image: &Path) -> Result<Vec<(u16, u16)>, String> {
    let wide = wides(&image.to_string_lossy());
    let module = unsafe { LoadLibraryExW(wide.as_ptr(), null_mut(), LOAD_LIBRARY_AS_DATAFILE) };
    if module.is_null() {
        return Err(format!("LoadLibraryExW failed (error {})", last_error()));
    }

    let mut found = Vec::new();
    for resource_type in [RT_GROUP_ICON, RT_ICON] {
        for id in 1..=MAX_ICON_ID {
            if find_blob(module, resource_type, id, LANG_NEUTRAL).is_some() {
                found.push((resource_type, id));
            }
        }
    }
    unsafe { FreeLibrary(module) };
    Ok(found)
}

/// Highest icon id searched for in a template image.
///
/// The template ships a handful of images and one group, all at low ids.
const MAX_ICON_ID: u16 = 0x40;
fn update_resource(
    update: HANDLE,
    resource_type: u16,
    name: ResourceId,
    language: u16,
    payload: &[u8],
) -> Result<(), String> {
    let data = if payload.is_empty() {
        null()
    } else {
        payload.as_ptr() as *const c_void
    };
    let ok = unsafe {
        UpdateResourceW(
            update,
            ResourceId::ordinal(resource_type).as_ptr(),
            name.as_ptr(),
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

/// Read the lowest-numbered icon group of `path`, plus the images it names.
///
/// The lowest id is what the shell picks when a file has no explicit icon
/// reference, so it is the icon a user expects to see.
pub(super) fn read_icon(path: &Path) -> Result<Option<IconSet>, String> {
    let wide = wides(&path.to_string_lossy());
    let module = unsafe { LoadLibraryExW(wide.as_ptr(), null_mut(), LOAD_LIBRARY_AS_DATAFILE) };
    if module.is_null() {
        // A target that cannot be mapped has no icon to take; not an error.
        return Ok(None);
    }

    let result = read_icon_from(module);
    unsafe { FreeLibrary(module) };
    result
}

fn read_icon_from(module: HMODULE) -> Result<Option<IconSet>, String> {
    // Probing ids in order rather than enumerating names: `EnumResourceNamesW`
    // reports only the first language of each resource, and a target whose
    // icons carry a non-English language reports none at all.
    let Some((group_id, group)) = first_group(module) else {
        return Ok(None);
    };

    let mut images = Vec::new();
    for id in referenced_image_ids(&group) {
        if let Some(bytes) = find_blob(module, RT_ICON, id, LANG_NEUTRAL) {
            images.push((id, bytes));
        }
    }

    if images.is_empty() {
        return Ok(None);
    }
    Ok(Some(IconSet {
        group_id,
        group,
        images,
    }))
}

/// The lowest-numbered icon group, searched across every language.
fn first_group(module: HMODULE) -> Option<(u16, Vec<u8>)> {
    // Resource ids are allocated from 1 and a real executable's icon groups sit
    // in the low hundreds at worst; a bounded scan beats an enumeration that
    // hides languages.
    for id in 1..=0x1000u16 {
        if let Some(bytes) = find_blob(module, RT_GROUP_ICON, id, LANG_NEUTRAL) {
            return Some((id, bytes));
        }
    }
    None
}

/// Look up one resource by id in a specific language.
///
/// `LANG_NEUTRAL` matches whatever language the image actually stores, which
/// is what a caller that does not care about localization wants.
fn find_blob(module: HMODULE, resource_type: u16, id: u16, language: u16) -> Option<Vec<u8>> {
    let resource = unsafe {
        FindResourceExW(
            module,
            ResourceId::ordinal(resource_type).as_ptr(),
            ResourceId::ordinal(id).as_ptr(),
            language,
        )
    };
    if resource.is_null() {
        return None;
    }

    let size = unsafe { SizeofResource(module, resource) } as usize;
    let loaded = unsafe { LoadResource(module, resource) };
    if loaded.is_null() || size == 0 {
        return None;
    }

    let data = unsafe { LockResource(loaded) };
    if data.is_null() {
        return None;
    }

    Some(unsafe { std::slice::from_raw_parts(data as *const u8, size) }.to_vec())
}

/// The `nId` of each image a group references, in directory order.
///
/// An `RT_GROUP_ICON` is a `GRPICONDIR` (6 bytes) followed by
/// `GRPICONDIRENTRY` records of 14 bytes; `nId` is the last `u16` of each.
pub(super) fn referenced_image_ids(group: &[u8]) -> Vec<u16> {
    const HEADER: usize = 6;
    const ENTRY: usize = 14;

    let Some(entries) = group.get(HEADER..) else {
        return Vec::new();
    };
    entries
        .chunks_exact(ENTRY)
        .filter_map(|entry| {
            let id = entry.get(12..14)?;
            Some(u16::from_le_bytes([id[0], id[1]]))
        })
        .collect()
}

/// Read a resource out of an image file by type and name.
pub(super) fn read(image: &Path, name: &[u16], resource_type: u16) -> Result<Vec<u8>, String> {
    let wide = wides(&image.to_string_lossy());
    let module = unsafe { LoadLibraryExW(wide.as_ptr(), null_mut(), LOAD_LIBRARY_AS_DATAFILE) };
    if module.is_null() {
        return Err(format!("LoadLibraryExW failed (error {})", last_error()));
    }

    let result = read_named_from(module, name, resource_type);
    unsafe { FreeLibrary(module) };
    result
}

fn read_named_from(module: HMODULE, name: &[u16], resource_type: u16) -> Result<Vec<u8>, String> {
    let resource = unsafe {
        FindResourceW(
            module,
            ResourceId::name(name).as_ptr(),
            ResourceId::ordinal(resource_type).as_ptr(),
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
            "FindResourceW(type {resource_type}) failed (error {code}) for the named resource"
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
        FreeLibrary(module);
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

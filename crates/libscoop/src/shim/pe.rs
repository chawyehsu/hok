//! The two PE header fields a shim needs: the subsystem, and where it lives.

const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
const IMAGE_NT_SIGNATURE: u32 = 0x0000_4550;

pub(super) const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
pub(super) const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;

/// Offset of `e_lfanew`, which points at the PE signature.
const DOS_LFANEW: usize = 0x3C;

/// Offset of `Subsystem` from the PE signature: 4-byte signature plus the
/// 20-byte COFF header plus 0x44 bytes of the optional header.
const PE_SUBSYSTEM: usize = 0x5C;

/// Upper bound on a plausible `e_lfanew`. The headers always live in the first
/// page, so anything beyond it is a bogus offset rather than a valid image.
const PE_PAGE: usize = 0x1000;

/// The subsystem field of a PE image, or `None` if the image is unparseable.
pub(super) fn subsystem(image: &[u8]) -> Option<u16> {
    let offset = subsystem_offset(image)?;
    let bytes = image.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Overwrite the subsystem field of a PE image in place.
pub(super) fn set_subsystem(image: &mut [u8], value: u16) -> Result<(), String> {
    let offset = subsystem_offset(image)
        .ok_or_else(|| String::from("shim template is not a valid PE image"))?;
    let bytes = image
        .get_mut(offset..offset + 2)
        .ok_or_else(|| String::from("shim template is truncated"))?;
    bytes.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn subsystem_offset(image: &[u8]) -> Option<usize> {
    let dos_signature = image.get(..2)?;
    if u16::from_le_bytes([dos_signature[0], dos_signature[1]]) != IMAGE_DOS_SIGNATURE {
        return None;
    }

    let lfanew = image.get(DOS_LFANEW..DOS_LFANEW + 4)?;
    let pe = i32::from_le_bytes([lfanew[0], lfanew[1], lfanew[2], lfanew[3]]);
    let pe = usize::try_from(pe).ok()?;
    if pe == 0 || pe >= PE_PAGE || pe + PE_SUBSYSTEM + 2 > image.len() {
        return None;
    }

    let signature = image.get(pe..pe + 4)?;
    if u32::from_le_bytes([signature[0], signature[1], signature[2], signature[3]])
        != IMAGE_NT_SIGNATURE
    {
        return None;
    }

    Some(pe + PE_SUBSYSTEM)
}

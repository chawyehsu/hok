//! A small growable UTF-16 string.
//!
//! Everything on the Windows side of a shim is UTF-16, and paths may contain
//! code units that are not valid UTF-8 (for example unpaired surrogates in a
//! file name). Converting through `str` at every step would corrupt them, so
//! the schema carries raw UTF-16 the whole way.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// A borrowed UTF-16 string.
#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Utf16Str {
    units: [u16],
}

/// An owned UTF-16 string.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Utf16String {
    units: Vec<u16>,
}

impl Utf16Str {
    /// Reinterpret a slice of code units as a string.
    ///
    /// No validity check is performed: every `u16` sequence is a legal
    /// Windows string, including unpaired surrogates.
    pub fn from_units(units: &[u16]) -> &Self {
        // SAFETY: `Utf16Str` is `#[repr(transparent)]` over `[u16]`, so the
        // pointer, length and alignment of the two agree, and the returned
        // borrow cannot outlive the source slice.
        #[allow(unsafe_code)]
        unsafe {
            &*(units as *const [u16] as *const Utf16Str)
        }
    }

    /// The raw code units.
    pub fn as_units(&self) -> &[u16] {
        &self.units
    }

    /// Length in code units.
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Whether two strings are equal ignoring ASCII case, which is the
    /// comparison Windows uses for environment variable names.
    pub fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        self.units.len() == other.units.len()
            && self
                .units
                .iter()
                .zip(other.units.iter())
                .all(|(a, b)| ascii_lower(*a) == ascii_lower(*b))
    }

    /// Number of bytes this string occupies in UTF-8, counting unpaired
    /// surrogates as their 3-byte encoding.
    pub fn utf8_len(&self) -> usize {
        self.units
            .iter()
            .map(|&unit| match unit {
                0x0000..=0x007F => 1,
                0x0080..=0x07FF => 2,
                _ => 3,
            })
            .sum()
    }

    /// Decode lossily for diagnostics only.
    ///
    /// Do not use the result to build paths or command lines.
    pub fn to_display_string(&self) -> String {
        String::from_utf16_lossy(&self.units)
    }
}

/// Fold one ASCII letter to lower case, leaving everything else alone.
fn ascii_lower(c: u16) -> u16 {
    if (u16::from(b'A')..=u16::from(b'Z')).contains(&c) {
        c + 32
    } else {
        c
    }
}

impl Utf16String {
    /// An empty string.
    pub fn new() -> Self {
        Self { units: Vec::new() }
    }

    /// Build from raw code units.
    pub fn from_units(units: impl Into<Vec<u16>>) -> Self {
        Self {
            units: units.into(),
        }
    }

    /// Build from UTF-8 text. Unpaired surrogates cannot be represented in
    /// UTF-8, so this is lossless.
    pub fn from_utf8(s: &str) -> Self {
        Self {
            units: s.encode_utf16().collect(),
        }
    }

    /// The raw code units.
    pub fn as_units(&self) -> &[u16] {
        &self.units
    }

    /// The raw code units, mutably.
    pub fn as_mut_units(&mut self) -> &mut Vec<u16> {
        &mut self.units
    }

    /// Borrow as an [`Utf16Str`].
    pub fn as_utf16(&self) -> &Utf16Str {
        Utf16Str::from_units(&self.units)
    }

    /// Length in code units.
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// Whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Append another string.
    pub fn push_utf16(&mut self, other: &Utf16Str) {
        self.units.extend_from_slice(other.as_units());
    }
}

impl core::str::FromStr for Utf16String {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_utf8(s))
    }
}

impl From<&str> for Utf16String {
    fn from(s: &str) -> Self {
        Self::from_utf8(s)
    }
}

impl From<&Utf16Str> for Utf16String {
    fn from(s: &Utf16Str) -> Self {
        Self::from_units(s.as_units())
    }
}

impl From<&Utf16String> for Utf16String {
    fn from(s: &Utf16String) -> Self {
        s.clone()
    }
}

impl PartialEq<Utf16Str> for Utf16String {
    fn eq(&self, other: &Utf16Str) -> bool {
        self.as_units() == other.as_units()
    }
}

impl PartialEq<Utf16String> for Utf16Str {
    fn eq(&self, other: &Utf16String) -> bool {
        self.as_units() == other.as_units()
    }
}

impl PartialEq<&Utf16Str> for Utf16String {
    fn eq(&self, other: &&Utf16Str) -> bool {
        self.as_units() == other.as_units()
    }
}

impl PartialEq<Utf16String> for &Utf16Str {
    fn eq(&self, other: &Utf16String) -> bool {
        self.as_units() == other.as_units()
    }
}

impl fmt::Debug for Utf16Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.to_display_string().as_str(), f)
    }
}

impl fmt::Debug for Utf16String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_utf16(), f)
    }
}

impl fmt::Display for Utf16Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_display_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn round_trips_through_utf8() {
        let s = Utf16String::from("C:\\Program Files\\7-Zip");
        assert_eq!(Utf16String::from(s.as_utf16()), s);
        assert_eq!(s.as_utf16().to_display_string(), "C:\\Program Files\\7-Zip");
    }

    #[test]
    fn preserves_unpaired_surrogates() {
        let units = [0x0041, 0xD800, 0x0042];
        let s = Utf16String::from_units(units);
        assert_eq!(s.as_units(), &units);
        // Lossy display must not panic on the unpaired surrogate.
        assert!(!s.as_utf16().to_display_string().is_empty());
    }

    #[test]
    fn compares_ascii_case_insensitively() {
        let a = Utf16String::from("PATH");
        let b = Utf16String::from("path");
        assert!(a.as_utf16().eq_ignore_ascii_case(b.as_utf16()));
        assert!(
            !a.as_utf16()
                .eq_ignore_ascii_case(Utf16String::from("PATHS").as_utf16())
        );
    }

    #[test]
    fn borrowed_form_round_trips() {
        let owned = Utf16String::from("hello");
        let borrowed = Utf16Str::from_units(owned.as_units());
        assert_eq!(owned, borrowed);
        assert_eq!(Utf16String::from(borrowed), owned);
    }

    #[test]
    fn reports_utf8_length() {
        assert_eq!(Utf16String::from("abc").as_utf16().utf8_len(), 3);
        assert_eq!(Utf16String::from("\u{00e9}").as_utf16().utf8_len(), 2);
        assert_eq!(Utf16String::from("\u{4e16}").as_utf16().utf8_len(), 3);
        // An unpaired surrogate is not encodable in UTF-8, but still occupies
        // three bytes in the WTF-8 form the text encoder emits.
        assert_eq!(Utf16String::from_units([0xD800]).as_utf16().utf8_len(), 3);
    }

    #[test]
    fn units_helpers_are_sound() {
        let mut s = Utf16String::from("ab");
        s.push_utf16(Utf16Str::from_units(&[0x00e9]));
        assert_eq!(s.len(), 3);
        assert_eq!(s.as_mut_units().len(), 3);
        assert!(!s.is_empty());
        assert!(Utf16String::new().is_empty());
        let _: Vec<u16> = s.as_units().to_vec();
    }
}

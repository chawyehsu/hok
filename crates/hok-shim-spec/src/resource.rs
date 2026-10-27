//! The binary encoding a shim launcher carries its configuration in.
//!
//! The configuration is a single `RT_RCDATA` resource: a self-describing byte
//! blob, read back with one `FindResourceW`/`LoadResource`/`LockResource` pass.
//!
//! A string table was the first design and it does not work: Windows keeps a
//! string table's ids 1..=16 in one process-wide 64 KiB pool that it merges
//! with the module's own table on every update, so a writer that adds a block
//! corrupts the resource directory of any image that already had one. Raw
//! resource data has no such merging, no id-space limit, and no length limit
//! beyond the resource section itself.
//!
//! Layout, all little-endian:
//!
//! ```text
//! magic   : 8 bytes   "hokshim\0"
//! version : u32       schema version, currently 1
//! target  : string    target path, required
//! cwd     : string    working directory, empty means inherit
//! flags   : u32       bit 0 = elevate
//! args    : count:u32 string*
//! env     : count:u32 (name:string value:string)*
//! ```
//!
//! A `string` is a `u32` count of UTF-16 code units followed by the units
//! themselves, with no terminator. Counts are never trusted blindly: every
//! read is bounds-checked against the remaining input.

extern crate alloc;

use alloc::vec::Vec;

use crate::config::Config;
use crate::utf16::Utf16String;

/// Marks a blob as a hok shim configuration.
pub const MAGIC: [u8; 8] = *b"hokshim\0";

/// Schema version a reader accepts. Writers bump it when the layout changes.
pub const SCHEMA_VERSION: u32 = 1;

/// `flags` bit requesting elevation.
const FLAG_ELEVATE: u32 = 1 << 0;

/// Failure while encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// The target path is empty, so the record could never launch anything.
    MissingTarget,
    /// A string is longer than the `u32` count prefix can describe.
    StringTooLong,
}

/// Failure while decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The blob is shorter than its header, or not a hok shim blob at all.
    NotAShimBlob,
    /// The version is one this build does not understand.
    UnsupportedVersion(u32),
    /// The stream ended in the middle of a field.
    Truncated,
    /// The target path is empty.
    MissingTarget,
}

/// Encode a configuration into its resource payload.
pub fn encode(config: &Config) -> Result<Vec<u8>, EncodeError> {
    if config.target.is_empty() {
        return Err(EncodeError::MissingTarget);
    }

    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
    write_string(&mut out, &config.target)?;
    write_string(&mut out, &config.cwd)?;
    let flags = if config.elevate { FLAG_ELEVATE } else { 0 };
    out.extend_from_slice(&flags.to_le_bytes());

    write_count(&mut out, config.args.len())?;
    for arg in &config.args {
        write_string(&mut out, arg)?;
    }

    write_count(&mut out, config.env.len())?;
    for (name, value) in &config.env {
        write_string(&mut out, name)?;
        write_string(&mut out, value)?;
    }

    Ok(out)
}

/// Decode a configuration from its resource payload.
pub fn decode(bytes: &[u8]) -> Result<Config, DecodeError> {
    let mut reader = Reader::new(bytes);

    let magic = reader.take(4)?;
    if magic != &MAGIC[..4] {
        return Err(DecodeError::NotAShimBlob);
    }
    let rest = reader.take(4)?;
    if rest != &MAGIC[4..] {
        return Err(DecodeError::NotAShimBlob);
    }

    let version = reader.u32()?;
    if version != SCHEMA_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }

    let target = reader.string()?;
    if target.is_empty() {
        return Err(DecodeError::MissingTarget);
    }
    let cwd = reader.string()?;
    let elevate = reader.u32()? & FLAG_ELEVATE != 0;

    let arg_count = reader.count()?;
    let mut args = Vec::with_capacity(arg_count.min(64));
    for _ in 0..arg_count {
        args.push(reader.string()?);
    }

    let env_count = reader.count()?;
    let mut env = Vec::with_capacity(env_count.min(64));
    for _ in 0..env_count {
        let name = reader.string()?;
        let value = reader.string()?;
        env.push((name, value));
    }

    Ok(Config {
        target,
        args,
        cwd,
        elevate,
        env,
    })
}

impl Config {
    /// See [`encode`].
    pub fn to_resource_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        encode(self)
    }
}

fn write_count(out: &mut Vec<u8>, count: usize) -> Result<(), EncodeError> {
    let count = u32::try_from(count).map_err(|_| EncodeError::StringTooLong)?;
    out.extend_from_slice(&count.to_le_bytes());
    Ok(())
}

fn write_string(out: &mut Vec<u8>, text: &Utf16String) -> Result<(), EncodeError> {
    write_count(out, text.len())?;
    for &unit in text.as_units() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(())
}

/// A bounds-checked cursor over the blob.
///
/// Every field is length-prefixed, so a corrupt or hostile blob must not be
/// able to make the parser read past its end or allocate an attacker-chosen
/// amount of memory up front.
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(DecodeError::Truncated)?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or(DecodeError::Truncated)?;
        self.offset = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A count of items that follow, clamped so a corrupt value cannot drive a
    /// huge allocation. Each item still consumes input, so the loop bound is
    /// enforced by `take` regardless.
    fn count(&mut self) -> Result<usize, DecodeError> {
        let count = self.u32()? as usize;
        let remaining = self.bytes.len() - self.offset;
        // A count cannot exceed the bytes left: every item needs at least one
        // byte. Reject early rather than loop over bogus counts.
        if count > remaining {
            return Err(DecodeError::Truncated);
        }
        Ok(count)
    }

    fn string(&mut self) -> Result<Utf16String, DecodeError> {
        let units = self.u32()? as usize;
        let byte_len = units.checked_mul(2).ok_or(DecodeError::Truncated)?;
        let bytes = self.take(byte_len)?;
        let mut text = Vec::with_capacity(units);
        for chunk in bytes.chunks_exact(2) {
            text.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        Ok(Utf16String::from_units(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            target: Utf16String::from("C:\\hok\\apps\\php\\current\\php.exe"),
            args: alloc::vec![
                Utf16String::from("--ini"),
                Utf16String::from("C:\\a b\\cli.ini"),
            ],
            cwd: Utf16String::from("C:\\Users\\me"),
            elevate: true,
            env: alloc::vec![(
                Utf16String::from("PHP_INI_SCAN_DIR"),
                Utf16String::from("C:\\hok\\apps\\php\\current\\conf.d"),
            )],
        }
    }

    #[test]
    fn round_trips() {
        let config = sample();
        let bytes = encode(&config).unwrap();
        assert_eq!(decode(&bytes).unwrap(), config);
    }

    #[test]
    fn round_trips_a_minimal_config() {
        let config = Config {
            target: Utf16String::from("C:\\x\\y.exe"),
            ..Config::default()
        };
        let bytes = encode(&config).unwrap();
        assert_eq!(decode(&bytes).unwrap(), config);
    }

    #[test]
    fn round_trips_many_entries() {
        let mut config = sample();
        config.args = (0u32..200).map(|_| Utf16String::from("x")).collect();
        for (index, arg) in config.args.iter_mut().enumerate() {
            let mut units = alloc::vec![u16::from(b'a')];
            for digit in alloc::format!("{index}").encode_utf16() {
                units.push(digit);
            }
            *arg = Utf16String::from_units(units);
        }
        config.env = (0u32..100)
            .map(|i| {
                (
                    Utf16String::from_units(
                        alloc::format!("VAR{i}").encode_utf16().collect::<Vec<_>>(),
                    ),
                    Utf16String::from("value"),
                )
            })
            .collect();

        let bytes = encode(&config).unwrap();
        assert_eq!(decode(&bytes).unwrap(), config);
    }

    #[test]
    fn rejects_a_foreign_blob() {
        assert_eq!(
            decode(b"not a shim").unwrap_err(),
            DecodeError::NotAShimBlob
        );
        assert_eq!(decode(b"").unwrap_err(), DecodeError::Truncated);
        assert_eq!(decode(b"hokshim").unwrap_err(), DecodeError::Truncated);
    }

    #[test]
    fn rejects_a_future_version() {
        let mut bytes = encode(&sample()).unwrap();
        bytes[8..12].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            decode(&bytes).unwrap_err(),
            DecodeError::UnsupportedVersion(99)
        );
    }

    #[test]
    fn rejects_a_truncated_blob_at_every_length() {
        let bytes = encode(&sample()).unwrap();
        for length in 0..bytes.len() {
            assert!(
                decode(&bytes[..length]).is_err(),
                "prefix of {length} bytes decoded"
            );
        }
    }

    #[test]
    fn refuses_to_encode_an_empty_target() {
        let config = Config::default();
        assert_eq!(encode(&config).unwrap_err(), EncodeError::MissingTarget);
    }

    #[test]
    fn rejects_a_bogus_count_without_allocating() {
        let mut bytes = encode(&sample()).unwrap();
        // Overwrite the argument count with a value far larger than the input.
        let args_at = 8 + 4 + 4 + sample().target.len() * 2 + 4 + sample().cwd.len() * 2 + 4;
        bytes[args_at..args_at + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        assert_eq!(decode(&bytes).unwrap_err(), DecodeError::Truncated);
    }

    #[test]
    fn preserves_unpaired_surrogates() {
        let mut config = sample();
        config.target = Utf16String::from_units(alloc::vec![0x0041, 0xD800, 0x0042]);
        let bytes = encode(&config).unwrap();
        assert_eq!(
            decode(&bytes).unwrap().target.as_units(),
            config.target.as_units()
        );
    }
}

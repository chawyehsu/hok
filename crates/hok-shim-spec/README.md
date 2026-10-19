# hok-shim-spec

The configuration schema shared by the `hok-shim` launcher and the `libscoop`
shim management module.

A shim launcher carries its configuration inside itself. This crate defines
what that configuration is, how it is laid out in a Windows string table
resource, and how the same configuration is rendered as Scoop's `.shim` text
file. It has no dependencies and does not touch the filesystem, the registry,
or Win32: the writer lives in `libscoop`, the reader in `hok-shim`, and this
crate is the contract between them.

## Layout

The configuration is one `RT_RCDATA` resource containing a little-endian blob:
an eight-byte magic (`hokshim\0`), a `u32` schema version, then the target
path, the working directory, a flags word, a counted list of arguments, and a
counted list of name/value environment pairs. Strings are a `u32` count of
UTF-16 code units followed by the units, with no terminator.

Every read is bounds-checked, so a truncated or corrupt blob fails with a clear
error rather than misparsing. Strings are raw UTF-16 throughout: paths on
Windows may contain unpaired surrogates, so converting through UTF-8 at any
step would be lossy.

## Text form

`to_text` renders the same configuration as a `.shim` file for Scoop
interoperability. The launcher never reads that file; `from_text` exists so
tests and inspection tooling can check what was written.
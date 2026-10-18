# Hok

Hok is a CLI implementation of Scoop in Rust. This glossary covers the vocabulary
of package installation and archive extraction, the area where Hok deliberately
diverges from Scoop's implementation while keeping its manifest semantics.

## Language

**Archive**:
A downloaded file that is opened to produce the contents of a package version
directory. An archive is a property of a download, not of a package.
_Avoid_: Package, installer, bundle

**Archive entry**:
One file, directory, or link stored inside an archive, identified by its path
relative to the archive root.
_Avoid_: Member, record, item

**Format**:
The container type that an archive actually is, as determined by the prober:
`zip`, `7z`, `rar`, `tar`, `msi`, `inno`, `nsis`, and so on.
_Avoid_: Extension, kind, type

**Declared format**:
The format implied by a download's file name before probing. It is a hint, never
authoritative.
_Avoid_: Extension, expected format

**Prober**:
The component that decides an archive's format from its name and a bounded
prefix of its bytes, without extracting it.
_Avoid_: Detector, sniffer, classifier

**Stub header**:
A format-specific signature appended after an executable stub, whose presence
identifies a format that has no header of its own at the start of the file. Its
position is evidence of the format, not a start offset: a stub-header format
probes as the whole file.
_Avoid_: Magic number, payload header, overlay signature

**Extractor**:
The component that writes one archive's entries into a destination directory.
An extractor is selected by the registry for a format, and may be a pure Rust
implementation or an externally installed binary.
_Avoid_: Unpacker, decompressor, handler

**Extractor registry**:
The single static table mapping formats to probers and to their ordered list of
extractors. It is the only place that answers "what can Hok extract?".
_Avoid_: Format table, dispatch table, factory

**Staging directory**:
The private directory where a package version is assembled, including all
extraction, before anything becomes live.
_Avoid_: Temp dir, partial dir, work dir

**Commit swap**:
The single rename that makes a fully assembled staging directory the live
package version directory.
_Avoid_: Move, promote, install, finalize

**Live package version directory**:
The directory named `apps/<package>/<version>` that package queries read and
that the `current` link points at.
_Avoid_: Install dir, target dir

**Extract observer**:
The subscriber that receives one callback per written archive entry during
extraction.
_Avoid_: Listener, callback, progress reporter

**Helper binary**:
An externally installed executable that some extractors delegate to, located
only through `PATH` and never installed automatically by Hok.
_Avoid_: Dependency, plugin, tool

**Path guard**:
The single check that makes an archive entry's destination stay inside its
destination directory.
_Avoid_: Sanitizer, validator, zip-slip check

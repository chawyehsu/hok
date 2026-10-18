# Archive extraction and package installation

Status: implemented. This document records what was built and the shape of the
interfaces, so later work extends it rather than rediscovering it. The design
rationale lives in `.agents/adr/0001`–`0005`; the vocabulary lives in `GLOSSARY.md`.

## Modules

```
crates/libscoop/src/internal/
├── archive/
│   ├── mod.rs        registry, Probe/Format, guard, extract orchestration
│   ├── probe.rs      bounded probing (1 KiB prefix, PE payload, ISO PVD)
│   ├── zip.rs        zip + nupkg
│   ├── sevenzip.rs   7z
│   ├── tar.rs        tar, compressed tar, bare single-file streams
│   ├── rar.rs        rar (single and multi-volume)
│   ├── msi.rs        msi (tables + embedded cabinets)
│   ├── inno.rs       Inno Setup installers
│   ├── nsis.rs       NSIS installers
│   ├── iso.rs        ISO9660 images
│   ├── cab.rs        cabinet (helper only)
│   └── helper.rs     PATH lookup and helper invocation
└── atomic.rs         staging directory, commit swap, sweep
```

## Public surface (`libscoop`)

Re-exported from the crate root:

```rust
pub fn probe(path: &Path) -> Fallible<Probe>;
pub fn is_archive_name(name: &str) -> bool;
pub fn extract(src: &Path, dest: &Path, options: &ExtractOptions<'_>) -> Fallible<ExtractReport>;
pub fn extract_options<'a>(
    observer: &'a dyn ExtractObserver,
    extract_dir: Option<&'a str>,
    extract_to: Option<&'a str>,
) -> ExtractOptions<'a>;

pub struct Probe { pub declared: Option<Format>, pub detected: Format, pub offset: u64, pub kind: ProbeKind }
pub enum Format { Zip, SevenZip, Tar, Gzip, Bzip2, Xz, Lzma, Zstd, Rar, Msi, Inno, Nsis, Iso, Cab, Lzh, Exe, Split }
pub trait ExtractObserver: Send + Sync { fn entry_written(&self, entry: &str); }
pub struct ExtractReport { pub probe: Probe, pub extractor: &'static str, pub entries: u64, pub bytes: u64, pub warnings: Vec<String> }
```

`extract` probes, runs the format's extractor chain into a temporary directory
inside the destination, applies `extract_dir`/`extract_to`, and only then moves
the result into place. A failure discards the temporary directory, so a retry
never sees a partial extraction.

The path guard rejects absolute paths, drive prefixes and `..` components. It is
the only path check; every extractor routes entries through it. A guard
rejection is final for the whole registry chain and is never retried with a
helper binary.

## Registry

`FORMATS: &[FormatSpec]` in `archive/mod.rs`, one entry per `Format`, is the only
place that lists supported formats. Each entry owns its name matcher, an optional
bounded-prefix detector, and an ordered `Ops { name, extract }` chain. To wire a
new extractor, point an `Ops` at it; to add a format, add an entry.

Chain order per format: pure Rust first, helper second where applicable.
`Format::tar.*` names are reported as `Format::Tar` and decoded according to the
name (`tar::compressed_container`), because probing cannot see inside a
compressed stream.

Extra 7z coders are enabled by adding features to the `sevenz-rust2` dependency,
not by touching dispatch.

## Atomic staging (`internal/atomic.rs`)

```rust
pub fn staging_dir(apps_dir: &Path, name: &str, version: &str) -> Fallible<StagingDir>;
pub fn commit_swap(live: &Path, staging: &StagingDir) -> Fallible<()>;
pub fn sweep(apps_dir: &Path, name: &str);
```

Staging lives at `apps/<name>/.staging/<version>`; `complete()` writes
`.staging/.complete`-style marker inside it; `commit_swap` retires an existing
live directory to `<version>.old` (unique tombstone on collision), promotes
staging, restores the retired directory if promotion fails, and sweeps retired
directories best effort.

## Installation (`package/sync.rs`)

`commit_package` per package, in dependency order:

1. `atomic::sweep`, then create the staging directory.
2. For each download: extract archives (pairing `extract_dir`/`extract_to` with
   a counter that only advances for extracted archives), copy non-archives under
   their URL leaf name.
3. Write `scoop-manifest.json` and `scoop-install.json`.
4. `complete()`, `commit_swap`, then point `current` at the version directory.

Already committed packages stay committed when a later package fails. Packages
using PowerShell scripts are refused with an explicit error rather than
installing without running their hooks.

## Events and CLI output

`Event::PackageExtractingStart(PackageExtractContext)`,
`Event::PackageExtractingEntry(String)` and `Event::PackageExtractingDone { .. }`
drive a single live line on stderr in `src/cmd/install.rs`. On a TTY the line is
redrawn in place and shortened to `...\<basename>` when too wide; when stderr is
not a TTY only per-archive completion lines are printed.

## Format coverage

| Format | Primary | Fallback |
|---|---|---|
| zip / nupkg | `zip` crate | `7z.exe` |
| 7z | `sevenz-rust2` | `7z.exe` |
| tar, tar.gz/bz2/xz/zst | `tar` + decoders | `7z.exe` |
| gz/bz2/xz/lzma/zst (bare) | decoders | `7z.exe` |
| rar | `rars` | `7z.exe` |
| msi | `msi` + `cab` tables | `msiexec /a`, then `7z.exe` |
| inno | `innospect` (Windows) | `innounp.exe` |
| nsis | `nsis` (Windows) | `7z.exe` |
| iso / img | `iso9660` | `7z.exe` |
| cab, lzh | — | `7z.exe` |

Declared but unsupported: header-encrypted 7z, base64-wrapped 7z. Both fail with
an explicit error instead of being skipped.

## Tests

`crates/libscoop/tests/archive.rs` exercises probing, extraction, `extract_dir`,
`extract_to`, entry reporting, non-archive rejection and path-traversal rejection
against committed fixtures in `tests/fixtures/archive/`. No test needs a helper
binary or network access.

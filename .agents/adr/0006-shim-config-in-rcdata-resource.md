# Shim configuration lives in an `RT_RCDATA` resource

status: accepted

A shim launcher must carry its per-shim configuration inside itself, so that
shims are self-contained and copyable. We store it as a single `RT_RCDATA`
resource holding a self-describing byte blob, injected at shim-creation time
with `BeginUpdateResourceW`/`UpdateResourceW`/`EndUpdateResourceW`.

## Considered Options

- **A string table (`RT_STRING`).** Rejected after it crashed every shim on
  the machine: Windows keeps a string table's ids 1..=16 in one process-wide
  64 KiB pool and merges the module's own table into it on every update, so
  writing a second block onto an image that already had one corrupts the resource
  directory. The failure is a hard access violation inside `LoadStringW`, not
  an error code. Avoiding it would mean deleting all existing resources first,
  which also discards the icon and version info we want to keep.
- **Appending a trailer after the PE image.** Simpler still, but leaves bytes
  outside any section: antivirus scanners and `signtool` dislike it, and we
  become responsible for all PE serialization.
- **A fixed linker section.** No parsing at all, but a compile-time size cap and
  manual section-offset arithmetic when patching.
- **Keeping the sidecar `.shim` as the real configuration.** Rejected by the
  goal: it inherits Scoop's runtime dependency on a second file, and two
  sources of truth invite drift.

## Consequences

- The OS rebuilds the resource section for us, so no hand-written PE writer is
  needed.
- The blob is ours to define, so there is no id space to run out of, no
  length limit beyond the resource section, and every field can be
  length-prefixed and bounds-checked. `hok-shim-spec` owns the format; the
  writer and the reader share it by construction.
- The first byte of the blob is a magic string and the next field a version,
  so a mismatched or corrupted shim fails with a clear message instead of
  misparsing.
- Icon injection shares the same resource-update session, which is why copying
  the target's icon is cheap.
- The `.shim` sidecar is still written for Scoop interoperability, but it is
  derived from the same configuration value and never read back.

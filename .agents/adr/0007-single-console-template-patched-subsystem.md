# One console shim template, GUI shims produced by patching the PE header

status: accepted

`hok-shim` is built exactly once, as a console-subsystem executable, and that
single binary is embedded into `hok.exe` and into `libscoop`. When creating a
shim for a GUI target, `libscoop` rewrites the two-byte `Subsystem` field of the
shim copy's PE optional header to `IMAGE_SUBSYSTEM_WINDOWS_GUI` (2).

## Considered Options

- **Two templates, one console and one GUI.** Rejected: it doubles the embedded
  payload and doubles the number of artifacts that must stay in sync, for a
  difference of one `u16`.
- **A single GUI-capable binary that self-detects and calls `FreeConsole`.**
  Cannot work as the only mechanism: a console-subsystem image launched from
  Explorer still gets a console window created before our code runs, so the
  flash is unavoidable without the header change.

## Consequences

- The template's own subsystem is authoritative for what the launcher does at
  startup: a GUI-subsystem instance has no console, so it calls
  `AttachConsole(ATTACH_PARENT_PROCESS)` when it has arguments to pass and
  otherwise stays detached; a console instance does neither.
- Changing the subsystem invalidates the PE checksum. Windows does not verify
  checksums for user-mode images, and this matches Scoop's own behavior, so the
  field is patched in place without recomputation.
- The shim launcher never needs to know its target's subsystem at launch time;
  the decision was baked into its header when it was created.

# Shim Support for 0.1.0

Plan for implementing EXE shim support. Scope is deliberately narrow; deferred
items are listed at the bottom so they are not silently forgotten.

Terminology follows `GLOSSARY.md`. The behavioral reference is the Scoop `Shim`
repository's `README.md` and `RATIONALE.md` (the specification four independent
implementations are tested against), not any single implementation.

## In scope

### `hok-shim-spec` (new crate)

A `no_std`, dependency-free crate shared by the template and by `libscoop`. It
owns the schema and nothing else:

- Resource layout constants: string-table block ids, slot ids, schema tag.
- The configuration record: target path, ordered arguments, working directory,
  elevation flag, environment overrides.
- UTF-16 slot encoding and decoding, matching what `UpdateResourceW` and
  `LoadStringW` expect.
- Encoding of the same record into the `.shim` text format.
- A schema/version tag in the first slot so a future format change is detected.

### `hok-shim` (new crate)

The template. One binary, console subsystem, embedded into `hok.exe`:

- Read its own image path, locate its configuration in its own resources.
- Resolve the target: expand environment variables, apply the dual meaning of
  `%~dp0` (the shim's own directory inside the target path, the target's
  directory everywhere else), reject leading path separators, normalize.
- Parse fixed arguments with `CommandLineToArgvW` semantics.
- Re-quote every argument it constructs, preserving the caller's original
  bytes rather than parsing and re-joining them.
- Apply environment overrides before spawning.
- Install the console control handler, create the job object, spawn
  suspended, assign to the job, resume.
- Handle `ERROR_ELEVATION_REQUIRED` by retrying through `ShellExecuteExW`.
- Seed standard handles from the creator's, falling back to `CONIN$`/`CONOUT$`.
- Propagate the child's exit code verbatim; use `1` for internal failures.
- Report diagnostics on stderr via `WriteConsoleW` or UTF-8 `WriteFile`,
  distinguished by whether stderr is a character device.
- Carry an `.rc` (via `winres` in `build.rs`): file description, product name,
  company, copyright, versions from `CARGO_PKG_VERSION`, original filename,
  a default icon, and a manifest with `longPathAware` and
  `consoleAllocationPolicy=detached`.
- A test-only diagnostic mode, behind a cargo feature, that prints its parsed
  configuration and exits. Never enabled in release artifacts.

Size and dependency optimization is explicitly out of scope for this pass.
Correctness first.

### `libscoop` shim module

- `ShimDef` (renamed from the existing `Shim`): a parsed `bin` entry.
- `Shim`: a created shim instance.
- Creating a shim: extract the embedded template, write the configuration
  resource, patch the subsystem for GUI targets, copy the target's icon
  groups, and write the Scoop `.shim` sidecar from the same configuration
  value.
- Deleting a shim: the existing removal logic, extended to handle the
  template-derived instance.
- Wiring `add` into the install pipeline. Today only `remove` is called, and
  `add` is commented out.

## Deferred

- `hok shim add/rm/list/info/alter` subcommands. The library module ships
  without CLI surface.
- Reading the `.shim` sidecar at runtime, under any circumstances.
- Rewriting `VS_VERSIONINFO` with the target's version data. The shim reports
  its own static version.
- Non-EXE shim types. Batch, bash, java, powershell and python targets keep
  producing script wrappers and are untouched.
- Multi-architecture templates. Each `hok` build embeds only the template for
  its own target triple. A 64-bit shim launches 32-bit targets fine; an
  architecture-specific shim is a separate feature.
- Migration of shims created by other implementations. None have shipped.
- Shim refresh on `hok update`. Existing shims keep the template they were
  created from.

## Verification

- `cargo build -p hok-shim && cargo build -p hok` in a clean tree.
- `cargo test -p libscoop`.
- `cargo clippy --all-targets -- -D warnings`.
- Test groups:
  1. Round-trip encoding of the configuration record and the `.shim` text.
  2. End-to-end: create a shim for a tiny target, assert the exit code
     propagates.
  3. Diagnostic mode: assert field parsing without launching a child.
  4. Subsystem patching: a GUI target produces a `Subsystem == 2` shim, a
     console target produces `3`.
- Manual: a shim for a GUI target starts without a console window flash.

## Build wiring

`libscoop`'s build script locates the `hok-shim` template, copies it into
`OUT_DIR`, and `include_bytes!`s it. When the template is missing it fails with
a message naming the command to build it. The profile preference is the profile
of the current build, not "release always".

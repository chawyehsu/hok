# hok-shim

The shim launcher template. A shim is a small executable in the shims
directory that runs another program on the user's behalf.

This crate builds exactly one binary, `hok-shim`, as a console-subsystem
image. That single artifact is embedded into `hok.exe`, and each shim is a
copy of it with two things changed: the per-shim configuration written into
its own string table resource, and, for a GUI target, the two-byte PE
subsystem field. There is no separate GUI binary; see ADR 0007.

## Behavior

At startup the launcher reads its configuration from its own `RT_STRING`
resource, not from a sidecar `.shim` file (`hok-shim-spec` defines that
layout). It then:

- decides its console policy from its own PE subsystem, not the target's;
- installs a console control handler that swallows every event, so the child
  receives them instead;
- applies the configured environment overrides to its own process;
- creates a job object that kills the child when the shim exits;
- starts the child suspended, assigns it to the job, then resumes it;
- falls back to a `runas` launch through the shell when the target demands
  elevation, or when the configuration asks for it;
- waits for the child and exits with its code verbatim. Internal failures
  exit `1` and report on stderr through `WriteConsoleW` or UTF-8 `WriteFile`,
  depending on whether stderr is a character device.

`%~dp0` is **not** expanded. The configuration is fully resolved by
`libscoop` when the shim is created, so every embedded path is already
absolute; expanding it again at launch could only disagree with what was
written.

## Build

`build.rs` uses `winres` to embed the version info, a default icon, and
`shim.manifest` (which carries `longPathAware` and
`consoleAllocationPolicy=detached`). The crate only builds on Windows.

## Diagnostic mode

```
cargo run -p hok-shim --features diagnostic -- --hok-diagnostic
```

Prints the decoded configuration and exits `0` without launching anything. The
feature is off by default and must never be enabled in shipped artifacts.

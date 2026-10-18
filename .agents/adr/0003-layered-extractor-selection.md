# Layered extractor selection with an explicit failure contract

Every format has a primary extractor and an ordered list of fallbacks. The
primary is a pure Rust crate wherever one exists, so extraction works with nothing
installed. A fallback is either a second Rust implementation or a helper binary
found on `PATH`. Selection is expressed as one static registry entry per format,
so adding a format or an encoder path is a data change rather than a change to
dispatch logic.

Falling back is deliberate and visible. A parser-level rejection may retry with
the next extractor in the chain; a partially written extraction never does.
Whichever extractor ran is reported in the extraction result and shown in output,
so a package that only works through a helper binary is diagnosable.

## Considered Options

- **Single extractor per format** — rejected: pure-Rust coverage is not yet
  complete for RAR, ISO and LZH, and refusing all of them would make Hok weaker
  than Scoop.
- **Helper binaries as the primary path** — rejected: it reintroduces the
  dependency on Scoop's helper packages and gives up the reason to use a Rust
  implementation.
- **Silent fallback on any error** — rejected: a corrupt archive would be retried
  with a second decoder, and a partially written destination would be retried on
  top of itself.

## Consequences

- Helper binaries are located only through `PATH`, never installed automatically.
  A missing helper is reported with the exact binary name and the package that
  provides it.
- Formats with no working extractor are declared unsupported and fail loudly
  rather than being skipped.
- The registry is the only definition of supported formats; nothing else may
  hard-code a format list.

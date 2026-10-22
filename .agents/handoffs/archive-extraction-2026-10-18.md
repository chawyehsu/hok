# Handoff: archive extraction and package installation

Status: **implemented, uncommitted, verified.** Working copy of the `hok` repo
(the `jj` change on top of `main` at `4bd02d1`) contains the whole change. Nothing
is committed yet.

## Read these first (do not re-derive them)

| Artifact | What it holds |
|---|---|
| `GLOSSARY.md` | Canonical vocabulary: Archive, Archive entry, Format, Declared format, Prober, Extractor, Extractor registry, Staging directory, Commit swap, Live package version directory, Extract observer, Helper binary, Path guard |
| `.agents/adr/0001-atomic-extraction-via-staged-swap.md` | Why staging + single rename commit |
| `.agents/adr/0002-magic-bytes-are-the-single-source-of-truth.md` | Why probing beats extensions |
| `.agents/adr/0003-layered-extractor-selection.md` | Registry shape, fallback rules, helper policy |
| `.agents/adr/0004-bounded-probing.md` | 1 KiB prefix + PE payload probing |
| `.agents/adr/0005-sequential-extraction.md` | Why extraction is not parallel |
| `.agents/plans/archive-implementation.md` | The shipped module map, public API, registry, atomic API, install flow, format coverage table, test layout |

`.agents/plans/archive-implementation.md` is the operational reference. This
document only adds what that file cannot: the traps hit during implementation and
the work that remains.

## What was built

- `crates/libscoop/src/internal/archive/` — registry (`mod.rs`), bounded prober
  (`probe.rs`), and one module per format: `zip`, `sevenzip`, `tar`, `rar`,
  `msi`, `inno`, `nsis`, `iso`, `cab`, `helper`.
- `crates/libscoop/src/internal/atomic.rs` — staging directory, commit swap with
  tombstoned retirement, sweep.
- `package/sync.rs::commit_package` — per-package assembly, extraction pairing,
  metadata files, swap, `current` link.
- `event.rs` — `PackageExtractingStart/Entry/Done` plus the `extract_options`
  constructor and the `ExtractObserver` re-export.
- `src/cui.rs::ExtractUI` and `src/cmd/install.rs` — one live stderr line on a
  TTY, per-archive completion lines otherwise.
- `crates/libscoop/tests/archive.rs` + `tests/fixtures/archive/` — 11 integration
  tests against committed fixtures.

Dependencies added to `libscoop`: `zip`, `tar`, `flate2`, `bzip2`, `lzma-rust2`,
`zstd`, `sevenz-rust2`, `rars = "=0.9.4"`, `msi`, `cab`, `iso9660`, plus
Windows-only `innospect` and `nsis`. `sha1`/`sha2` were bumped to `0.11` (see
the first trap below).

Verification at handoff time:

```
cargo fmt --all -- --check          clean
cargo clippy --workspace --all-targets   clean
cargo test --workspace             120 lib + 11 integration + 2 doc tests pass
```

## Traps hit during implementation

1. **`rars` 0.9.4 pins newer RustCrypto crates.** It requires `sha1 0.11`,
   `sha2 0.11`, `aes 0.9`, `hmac 0.13`. `internal/hash.rs` needed a one-line fix
   because the digest API changed how you format the digest. If you pin a
   different `rars` version, re-check the whole hash module.
2. **`sevenz-rust2` panics or fails on header-encrypted archives** instead of
   reporting an error. Detection of `0x17` headers from a prefix is not reliable
   (a `-mhc=on` archive can produce a header CRC that looks like an ordinary
   header, and a BCJ-filtered payload can contain a literal `7z` signature at a
   non-boundary offset). Header-encrypted archives are therefore declared
   unsupported and surface as a parse error. Do not add a heuristic that guesses.
3. **`.tar.gz` cannot be detected from bytes within the probe budget.** The
   compressor hides the tar header, so the *declared name* decides:
   `compressed_tar()` in `archive/mod.rs` maps a declared compressor plus a
   `.tar.*`/`.tgz`/`.tbz2`/`.txz`/`.tzst` name to `Format::Tar`, and
   `tar::compressed_container()` picks the decoder from the same name. If you add
   a new compressed-tar suffix, update both functions.
4. **ZIP entry names are sanitized by the `zip` crate.** `ZipFile::name()`
   returns a *sanitized* `Result<Cow<str>>` (it strips `..`), so a traversal check
   built on it silently passes. The pre-flight check in `zip::extract` uses
   `by_index_raw` + `name_raw()` on purpose. Do not "simplify" that loop.
5. **A path-guard rejection must not fall through to a helper.** The registry
   chain retries the next extractor on failure, and `7z.exe` will happily extract
   `../evil`. `Error::is_unsafe_archive()` marks guard rejections and both the
   chain and `zip::extract_with_fallback` stop on it. Any new extractor must
   propagate the guard error unchanged (no rewording) or this check breaks.
6. **Guard must normalise separators before inspecting components.** On Windows,
   `Path::new(r"..\..\evil").components()` does not yield `ParentDir`, so `guard`
   replaces `\` with `/` first.
7. **`rars` locates the RAR signature from the start of the file**, so a
   self-extracting RAR cannot be parsed at an offset. `rar::parse_at` strips the
   stub into a temporary file (`TempArchive`) and parses that. The temp file is
   removed on drop; keep the guard object alive for the whole extraction.
8. **`sevenz-rust2` and `rars` do not report bytes written.** Both were given a
   counting writer so `ExtractReport::bytes` is real. If you replace a writer
   path, keep the counting.
9. **ZIP/RAR DOS timestamps are local time.** Tests assert stored archive values
   or tolerances instead of comparing an epoch computed from the local clock.
   Do not reintroduce exact-epoch assertions.
10. **Probing for a PE payload cannot assume the payload starts at `e_lfanew`.**
    The prober probes the window at `e_lfanew`, then the region after the last
    section, and validates each candidate offset with the target parser.
11. **Only declaration-based detection produces `Format::Inno` / `Format::Nsis`.**
    They have no independent magic. A `.exe` download is routed to them only when
    the name matches or a manifest says so; otherwise it is `Format::Exe` and
    `extract` refuses it with a clear message.

## Known gaps and deliberate omissions

- **PowerShell-script packages are refused, not installed.** `commit_package`
  returns an explicit error for `pkg.has_install_script()`. That is intentional
  until PS hosting exists (item in `chawyehsu/hok#15`).
- **Shims, shortcuts, environment variables and persist are not created.** Only
  the version directory, metadata files and the `current` link.
- **Uninstall rollback does not exist.** `atomic` can retire a directory, but
  `sync::remove` still deletes in place. A retired-directory swap for uninstall
  was designed and explicitly deferred.
- **Cross-package rollback does not exist.** A committed package stays committed
  when a later package in the same command fails (ADR 0001).
- **Extraction is sequential per archive.** Per-package parallelism was designed
  and deferred (ADR 0005).
- **Helpers are `PATH`-only and never auto-installed.** A missing helper yields
  `Error::HelperNotFound` naming the exact binary and the Scoop package that
  provides it.
- **`cab` and `lzh` have no pure-Rust path.** They require `7z.exe`.
- **`innospect`/`nsis` are Windows-only.** On other targets those formats report
  the crate is unavailable and fall back to the helper.
- **No cancellation.** Ctrl-C is not wired; a global handler was designed and
  deferred.
- **`upgrade.rs` still prints `Not implemented yet.`** The install path is wired;
  upgrade's command body is not.

## Next work, in the order it probably matters

1. Commit the change (the repo uses `jj`; the AGENTS.md VCS contract applies).
   Split it: docs/ADRs, archive module, atomic module, install wiring, CLI, tests.
2. Run a real `hok install` against a real bucket package and inspect the live
   line rendering on Windows Terminal. The integration tests cover extraction,
   not terminal output.
3. Wire `hok upgrade` through the same `commit_package` path (it currently stops
   before installation).
4. Implement uninstall retirement + rollback using `internal::atomic`
   (retire the live dir to `.staging/<version>.old`, delete the rest, restore on
   failure, sweep best effort).
5. Add installation-level tests: a fake archive, a two-package dependency chain,
   a package whose extraction fails (assert the live directory was never created),
   and a package with `extract_dir`/`extract_to`.
6. Consider per-package extraction parallelism behind an explicit switch, gated
   on real-world timing data.
7. Revisit `rars`: pinning `=0.9.4` is a licensing choice (dual MIT/Apache at
   0.9.4, Apache-only from 0.10). Revisit if upstream restores dual licensing.
8. Add a `.exe` SFX archive case to the fixtures (currently only zip/7z/tar/rar
   coverage is exercised end to end).

## Suggested skills for the next session

- `jujutsu` — the repository uses `jj`; use it to split and describe the change.
- `codebase-design` — the `Extractor`/`Ops` seam and `internal::atomic` are the
  two places further work will land; use its vocabulary when shaping them.
- `domain-modeling` — `GLOSSARY.md` already exists; extend it only if a new
  domain term appears (e.g. uninstall retirement).
- `grill-with-docs` — only if a new ADR-worthy decision appears, such as the
  uninstall rollback contract.
- `handoff` — to produce the next handoff when this work is finished.

## Constraints to respect

- Rust edition 2024, stable toolchain, no `unsafe` in new code.
- `libscoop` errors are `thiserror` variants in `error.rs`; new failure modes get
  a variant, not a `String`.
- Code comments, docs and commit messages in English.
- No `TODO`/`FIXME` markers in finished code.
- Helper binaries are found on `PATH` only; never auto-install them.

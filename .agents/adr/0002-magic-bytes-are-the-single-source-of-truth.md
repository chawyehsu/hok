# Magic bytes are the single source of truth for archive format

Scoop picks an extractor purely from the download's file name, and silently
skips anything it does not recognise. That produces empty installs and
mis-parsed installers whenever a bucket ships an archive under a misleading or
extensionless name. Hok probes the archive instead: a bounded prefix of the file
plus the file name decide the format, and the probe result — not the name — drives
extraction.

A probe returns the detected format, the declared format, and the byte offset
where the archive actually starts. The offset is how self-extracting archives are
handled: a payload behind an executable stub is extracted relative to that offset
instead of being read into memory or split into a temporary file.

## Considered Options

- **Extension only, as Scoop does** — rejected: the failure mode is silent. An
  archive that is copied instead of extracted yields a package that installs but
  does not run.
- **Full-file magic scan** — rejected: it can lock onto a nested archive's
  signature inside a compressed payload (a 7z payload whose contents include an
  `.rar` contains the RAR signature verbatim), and it costs a full read before any
  work begins.
- **Content-only, ignoring the name** — rejected: the name still carries the
  user's declared intent and is the only thing available for formats without a
  reliable signature.

## Consequences

- A file whose name claims a known archive format but whose bytes disagree is an
  error, not a copy. The error reports both the declared and the detected format.
- Files that are neither archive-named nor archive-looking are copied verbatim;
  that is what makes script-only and supplementary downloads work.
- Every extractor that can honour a start offset must accept one, so the probe
  result type must always carry it.

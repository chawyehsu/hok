# Extraction stays sequential

Archives are extracted one entry at a time, in archive order. The observer
receives an entry callback as each file is written, so the file names shown during
extraction are the ones actually written, and solid archives behave correctly.

This matches Scoop's extractors and the reference implementation that informed the
output design, neither of which parallelises extraction.

## Considered Options

- **Parallel extraction per entry** — deferred: it decodes entries out of order,
  which breaks solid archives and turns the live file-name display into an
  unordered stream. It also multiplies peak memory.
- **Parallel extraction per archive** — allowed only at the package level: two
  independent archives in the same package may be extracted concurrently, because
  they share no entries. This is a scheduling decision made above the extractor,
  not inside it.

## Consequences

- Progress display is monotonic and truthful: entry N is reported after entries
  before it were written.
- Extraction throughput is bounded by a single core per archive. If that becomes
  the dominant cost, parallelism belongs in the package scheduler, behind an
  explicit switch, not in the extractor.

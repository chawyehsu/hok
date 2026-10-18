# Atomic extraction via staged swap

Scoop extracts directly into `apps/<name>/<version>` and aborts on failure,
leaving a half-populated version directory that still looks installable. Hok
assembles every package version in a private staging directory (one sibling per
package, keyed by version) and commits it with a single same-volume rename, so a
failed or killed extraction leaves the live directory untouched.

The swap is the only commit point. Replacing an existing version (upgrade,
replace) stages the new version first, then renames the old live directory out
and the staging directory in; a failed second rename restores the old directory.
Retired directories are removed best-effort and, if still locked by a running
binary, shunted to a unique tombstone so the next swap is never blocked.

## Considered Options

- **Extract in place, delete on failure** — rejected: the primary failure mode is
  a locked executable, which makes the cleanup itself fail and leaves a live
  directory that half-exists. Scoop does this today.
- **One staging directory per transaction** — rejected: a per-package staging
  directory keeps the commit granularity equal to the install unit and lets one
  package commit while another is still being assembled.
- **Rollback of committed packages** — deferred: cross-package rollback needs a
  durable journal, and per-package atomicity already gives the property that a
  failure never damages a working install.

## Consequences

- Staging, retired and live directories are same-volume by construction, which
  is what makes the rename atomic on Windows.
- A committed package cannot be rolled back by a later failure in the same
  command; already-committed packages stay installed, as in Scoop.
- Leftover staging and tombstone directories must be swept on the next
  operation; they are never mistaken for installed packages.

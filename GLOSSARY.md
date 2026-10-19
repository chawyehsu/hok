# Glossary

Terms shared across hok. This file is a glossary: it defines what words mean
here. It is not a specification and contains no implementation decisions.

## Package management

**Package** — A software entry described by a manifest, identified by a bucket
and a name. A package may be installed at one or more versions.

**Manifest** — The JSON description of a package: where to download it, how to
install it, and what it exposes to the user.

**Bucket** — A collection of manifests distributed together, usually as a git
repository.

**Installation** — A specific package at a specific version, materialized under
the hok root's `apps/` directory.

## Shims

**Shim** — A small launcher executable placed in the shims directory that runs
another program on the user's behalf. Invoking the shim runs the target program.

**Shim definition** — An entry in a package manifest's `bin` field describing
what to shim: the target file, optionally a different name, and optionally
fixed arguments. A shim definition is a *request*; no files exist until it is
materialized.

**Shim instance** — One materialized shim in the shims directory. A shim
instance owns its launcher executable and any companion files. "Instance" is
only used where the distinction from the definition matters.

**Shim launcher** — The executable part of a shim instance: the thing the shell
actually executes. Named `hok-shim` at the crate level; at runtime it carries
the shim's own name, not the string "hok-shim".

**Shim template** — The single unconfigured launcher binary from which every
shim launcher is produced. One template serves all shims; each shim launcher is
a copy of it with configuration applied.

**Shim metadata** — The `.shim` text file sitting next to a shim launcher,
describing the same target in Scoop's format. It exists for interoperability
with Scoop. A hok shim launcher does not read it.

**Embedded configuration** — The per-shim data carried inside the shim
launcher itself. Distinct from shim metadata: embedded configuration is what
the launcher consumes, shim metadata is what other tools consume.

**Shim type** — The category of a shim derived from its target's file
extension: executable, batch, bash, java, powershell, or python. Only the
executable type produces a shim launcher; the others produce script wrappers.

**Alternate shim** — A shim instance of a given name belonging to a package
that does not currently own that name. Alternates are kept alongside the owning
instance so that removing the owner can promote another package's version.
Scoop suffixes them as `name.package`; they are not otherwise special.

**Shim conflict** — The situation where installing package A wants a shim name
that package B currently owns.

**Target** — The program a shim launches, as resolved to an absolute path at
creation time. A target is never resolved at launch time.

**Target directory** — The directory containing a shim's target. It is the
`%~dp0` of everything inside the shim except the shim's own location.

## Windows

**Subsystem** — The field in a PE image's optional header declaring whether the
image wants a console (CUI) or not (GUI). Determines whether Windows allocates a
console for the process at startup.

**Elevation** — Running a program with higher privilege than the caller,
requiring the user to approve a UAC prompt.

**Job object** — A Windows kernel object used to group processes so that they
can be controlled and reaped together. A shim uses one to tie its child's
lifetime to its own.

**Console control event** — A notification (Ctrl+C, Ctrl+Break, close, logoff,
shutdown) delivered by Windows to every process attached to a console. A shim
receives the same events as its child because they share a console.

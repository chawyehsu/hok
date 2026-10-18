# Bounded probing instead of full-file scanning

The prober reads at most one kilobyte from the front of an archive. If that prefix
is a PE image, it follows the image's own header field to locate any appended
payload and probes from there. Nothing else is scanned, and no archive is read in
full before extraction begins.

This bound keeps memory constant regardless of archive size and keeps probing
cheap for the multi-gigabyte installers Scoop buckets routinely ship.

## Considered Options

- **Scan the whole file for signatures** — rejected: a compressed payload can
  contain a verbatim copy of another format's signature, so a full scan can select
  the wrong offset. It also reads the entire archive before extraction starts.
- **Read the whole prefix to find the payload offset** — rejected: for a large
  self-extracting archive that means holding the stub in memory.
- **Trust a payload offset claimed by the stub without validation** — rejected:
  the offset must still produce a header that the target parser accepts.

## Consequences

- A format whose signature is not within the prefix and is not reachable through a
  PE stub will not be detected.
- Headers that are themselves encrypted cannot be recognised from the prefix; such
  archives are reported as unsupported rather than guessed at.
- Validation of the probe result requires the format's parser, so the prober and
  the registry must be able to consult each other.

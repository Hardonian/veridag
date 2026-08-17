# Veridag Protocol

This directory is **Level 1**: the normative, implementation-independent protocol
specification.

* `specification/` — the numbered normative documents. Start at `00-overview.md`.
* `schemas/` — machine-readable schemas (added as they stabilize).
* `test-vectors/` — golden vectors every implementation must reproduce.
* `conformance/` — conformance harness notes.
* `versions/` — per-protocol-version snapshots.

Authority order: Level 1 (here) > Level 2 (`formal/quint/`) > Level 3
(`implementations/`). An implementation is correct only if it satisfies Levels 1
and 2.

Change process: see `specification/17-upgrades.md`. Every consensus-visible change
starts here, not in code.

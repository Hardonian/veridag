# Veridag industry packs

Industry packs map established external standards into the same
privacy-preserving `EvidenceAnchor`. They do not fork consensus and they do not
store regulated source documents on the shared ledger.

Each pack defines its normative standard, accepted media types, adapter name,
and maturity. The Rust `veridag-industry` crate performs bounded admission
checks and produces deterministic anchor bytes. Complete certification remains
the responsibility of the pack's standard-specific conformance suite.

Pack promotion follows `docs/capability-matrix.md`.


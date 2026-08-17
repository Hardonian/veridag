# Veridag Formal Model (Level 2)

Quint model of BaselineDagBft (spec 09-consensus). This is the executable formal
model; the normative spec is Level 1 (`protocol/specification/`), and Rust is the
reference implementation (Level 3).

## Files

| File | Role |
|------|------|
| `consensus.qnt` | The parameterized DAG-BFT model: vertices, parents, equivocation, asynchronous delivery, quorum commit rule, `nextWave` commit cursor. Invariants: `Agreement`, `Finality`, `Integrity`. |
| `instance4.qnt` | Instantiation for n=4, f=1 (`Set(1,2,3,4)`, Byzantine `Set(4)`). |
| `consensus_test.qnt` | Deterministic run test that constructs a coherent DAG and forces a wave-1 commit, asserting all invariants on the committed path. |
| `basicSpells.qnt` | Standard Quint spells (vendored from informalsystems/quint). |
| `traces/` | Minimized counterexample traces (empty: no invariant violated to date). |

## Model scope

Safety of the commit rule, abstracting transaction contents (ordering safety does
not depend on them). Honest validators propose constructively (frontier+1,
referencing all delivered frontier vertices); Byzantine validators equivocate and
propose arbitrary valid-shaped vertices; delivery is fully asynchronous
(delay/reorder/duplicate/withhold). `Integrity` requires committed anchors to be
real proposed vertices of the right wave and author.

## How to run

```bash
# typecheck
quint typecheck consensus.qnt
quint typecheck instance4.qnt

# deterministic commit-path test (non-vacuity)
quint test consensus_test.qnt

# randomized invariant checking (n=4, f=1)
quint run instance4.qnt --invariant=Agreement --max-steps=30 --max-samples=500
quint run instance4.qnt --invariant=Finality  --max-steps=30 --max-samples=500
quint run instance4.qnt --invariant=Integrity --max-steps=30 --max-samples=500
```

## Verification status (honest)

* `quint typecheck` — clean.
* `quint test consensus_test.qnt` — passes: the commit rule is reachable and
  Agreement/Finality/Integrity hold on a concrete committed path (non-vacuous).
* `quint run` randomized simulation, 500 traces × 30 steps, per invariant — no
  violation found for Agreement, Finality, Integrity.
* `quint verify` (Apalache bounded model checking) — currently blocked by
  Apalache's handling of `List.length()`/`.indices()` in state-level invariants
  ("Expected a constant integer expression. Found: Len(...)"). This is a known
  Apalache limitation with dynamic-list invariants, not a model bug. Full bounded
  verification is queued behind re-encoding invariants over bounded sequences or
  a TLA+ translation; the randomized simulator and deterministic test provide the
  current safety evidence.

Do not claim "formally verified" beyond what the above demonstrates. We claim:
the commit rule is modeled, typechecked, exercised on a committed path, and
invariant-checked by randomized simulation; exhaustive bounded verification via
Apalache is in progress.

## Model-based testing (Phase 1 → Phase 7 bridge)

Formal traces feed implementation tests: a Quint/Apalache trace is normalized into
Rust simulator events (`implementations/rust/crates/simulator`, Phase 7), executed
against the reference implementation, and checked against the same invariants.
This keeps the model tied to code, not documentation.

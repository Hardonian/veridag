# 09 — Consensus (BaselineDagBft)

Status: NORMATIVE

BaselineDagBft is a DAG-based BFT consensus in the family of
Narwhal/Bullshark/Shoal: a structured DAG with a deterministic leader schedule and
a wave-based commit rule. It is deliberately simple; optimized engines are
post-v0.1 (21-consensus-engine-abstraction).

## Assumptions

* Validator set is a `StaticCommittee` of `n` validators with uniform weight
  (16-validator-membership).
* `n >= 3f + 1` where `f` is the number of Byzantine validators tolerated.
* Safety holds without accurate clocks; liveness holds under eventual synchrony.

## Structure

* **Wave** = `WAVE = 4` consecutive rounds.
* **Anchor** of wave `w` = the vertex authored in round `w * WAVE` by the
  deterministic leader `L(w) = validators[ (w * WAVE) mod n ]`.

## Commit rule (MUST)

A validator determines anchors and their fate using its local DAG:

1. For each wave `w`, if the anchor `A(w)` is present in the local DAG, decide it
   `Commit` if there exist at least `2f + 1` vertices in round `w*WAVE + 1` that
   reference `A(w)` as a parent. Otherwise mark it `Undecided` for now.
2. **Vote interpretation across waves (Shoal-style pipelining):** when deciding
   anchor `A(w)` for the latest wave, also revisit the anchor of the previous
   wave `A(w-1)`: if `A(w)` commits and `A(w-1)` is referenced by the causal
   history of the vertices that voted for `A(w)`, then `A(w-1)` also commits;
   otherwise `A(w-1)` is skipped (its transactions will be included by later
     anchors if still referenced).
3. When an anchor commits, totally order the committed anchors by wave number.
   For each committed anchor, perform a deterministic causal traversal of the
   DAG from that anchor, collecting all vertices not yet ordered by any earlier
   committed anchor.
4. Within a wave's ordered vertex set, produce the ordered transaction list per
   10-ordering.

The commit decision is a pure function of the local DAG. Two honest validators
with the same DAG MUST derive the same commit sequence. The formal model
(`formal/quint/consensus.qnt`) proves that two honest validators never commit
conflicting histories even when their local DAGs differ transiently.

## Safety invariants (MUST; proved in the formal model)

* **Agreement** — no two honest validators finalize conflicting checkpoints.
* **Finality** — an honest validator never reverts finalized state.
* **Integrity** — no invalid state transition is finalized.
* **No conflicting finalized histories** — finalized prefixes are consistent.

## Liveness (SHOULD)

Under eventual synchrony and with fewer than `f + 1` crashed/Byzantine validators,
anchors are eventually committed and the DAG keeps growing. The protocol does not
rely on timeouts for safety; timeouts MAY be used by implementations only to
trigger retransmission, never to justify a commit.

## Engine abstraction

The reference implementation exposes `ConsensusEngine`; v0.1 defines only
`BaselineDagBft` as normative. Exactly one engine is normative per protocol
version, so runtime disagreement about consensus semantics is impossible.

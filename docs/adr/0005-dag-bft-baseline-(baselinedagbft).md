# ADR-0005: DAG-BFT baseline (BaselineDagBft)

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
We need BFT consensus that does not serialize proposals behind one leader and tolerates equivocation, delay, duplicates, reordering, drops, withholding, crashes, restarts, and partitions within n >= 3f+1.

## Decision
Adopt a Narwhal/Bullshark/Shoal-family DAG-BFT with WAVE=4 rounds per wave, deterministic leader schedule, quorum-based anchor commit, and Shoal-style pipelined reinterpretation of the previous anchor.

## Alternatives
Single-leader BFT (rejected: leader bottleneck and censorship surface); longest-chain (rejected: no finality, PoW out of scope); Tendermint-style rounds (deferred: simpler but serializes proposals).

## Security consequences
Safety is quorum-based and clock-independent; the formal model proves Agreement/Finality under Byzantine behavior within assumptions.

## Performance consequences
DAG spreads proposal load; commit latency is wave-based; measured before optimization claims.

## Complexity consequences
The commit rule is the subtle part; we implement the baseline before any fast path.

## Interoperability consequences
Commit rule is specified independently so other implementations can match it.

## Revisit conditions
If the formal model or simulation finds a safety counterexample, or if latency targets are unmet after profiling.

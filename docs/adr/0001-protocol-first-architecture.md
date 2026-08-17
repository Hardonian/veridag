# ADR-0001: Protocol-first architecture

Status: Accepted (v0.1)
Date: 2026-08-17

## Context
Multiple independent implementations must eventually agree on identical history and state commitments. If the Rust code becomes the source of truth, independent implementations are impossible.

## Decision
Maintain three levels: normative Markdown spec (Level 1), Quint formal model (Level 2), reference implementations (Level 3). Correctness = satisfying Levels 1 and 2. Every consensus-visible change flows spec -> model -> vectors -> code.

## Alternatives
Code-as-spec (rejected: locks to one language); spec-after-the-fact (rejected: drift).

## Security consequences
Forces security-relevant semantics to be written down and reviewed before implementation.

## Performance consequences
None at runtime; adds process cost.

## Complexity consequences
Adds process overhead; pays for itself at the first cross-implementation test.

## Interoperability consequences
Enables veridag-go / veridag-zig / veridag-cpp without reading Rust internals.

## Revisit conditions
If the project permanently collapses to a single implementation and abandons independence.

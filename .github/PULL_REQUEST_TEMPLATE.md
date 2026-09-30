# Pull Request

## What & Why

<!-- 1–3 sentences describing what this PR does and why it's needed. Link to the issue if applicable: Closes #123 -->

## Type of change

- [ ] Bug fix (non-breaking)
- [ ] Feature / new capability
- [ ] Protocol change (requires spec + formal model + vector updates)
- [ ] Refactor / performance improvement
- [ ] Documentation only
- [ ] CI / tooling

## Checklist

- [ ] `just check` passes locally (fmt + clippy + test + doc)
- [ ] New behaviour is covered by tests
- [ ] No `unwrap()` on attacker-controlled input
- [ ] No `unsafe` added
- [ ] If protocol-visible: spec updated, formal model passes, vectors regenerated (`just vectors`)
- [ ] CHANGELOG.md updated under `[Unreleased]`

## Testing done

<!-- Describe how you verified the change works: unit tests, demo, devnet, simulation -->

## Notes for reviewer

<!-- Anything the reviewer should pay special attention to -->

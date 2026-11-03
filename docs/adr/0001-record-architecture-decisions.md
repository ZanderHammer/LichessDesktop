---
status: accepted
date: 2026-11-03
---

# Record architecture decisions as ADRs

## Context and Problem Statement

Decisions about the stack, release process and security policy need to be traceable for contributors and AI agents. They should not bloat `CLAUDE.md` or get lost in commit history.

## Considered Options

* Architecture Decision Records in `docs/adr/` (MADR format)
* Put the decisions in `CLAUDE.md` / `README.md`
* No written record

## Decision Outcome

Chosen option: "Architecture Decision Records in `docs/adr/` (MADR format)", because each decision gets its own reviewable file with context and consequences, and `CLAUDE.md` can stay lean and just link here.

### Consequences

* Good, because the reason behind a decision survives after the people who made it leave.
* Good, because a decision is superseded by a new ADR instead of being silently edited.
* Bad, because it is a small extra step when making a decision.

## Process

* Copy `0000-template.md` to `NNNN-kebab-title.md`, using the next free number.
* Accepted ADRs are not rewritten. To change a decision, write a new ADR and set the old one's status to `superseded by ADR-NNNN`.

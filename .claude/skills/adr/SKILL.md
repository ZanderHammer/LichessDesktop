---
name: adr
description: Create or supersede an Architecture Decision Record in docs/adr (MADR format). Use when a technical or process decision is made or changed, or when the user asks to record or document a decision.
---

# ADR

Decisions live in `docs/adr/` (ADR 0001) and not in CLAUDE.md.

## New ADR
1. Find the next number: the highest `NNNN-` prefix in `docs/adr/` + 1, zero-padded to four digits.
2. Copy `docs/adr/0000-template.md` to `docs/adr/NNNN-short-kebab-title.md`.
3. Fill in the frontmatter (`status: accepted` or `proposed`, `date: YYYY-MM-DD` today), the context, the considered options, the decision and the consequences.
4. Back the context with sources: link the docs and measurements you looked up. Don't guess.
5. Commit on its own: `docs: add ADR NNNN (<short title>)`.

## Change a decision
- Don't rewrite accepted ADRs. Write a new ADR, then change the old one's status to `superseded by ADR-NNNN` in the same commit.

---
id: 41
title: Filter by typing
type: feature
status: backlog
milestone: v0.3
depends_on:
- 20
- 40
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: filter
---

## Problem

Facets answer the questions the tool anticipated. "Where is that thing called
something like postgres" is not one of them.

## Proposal

`/` opens a filter that narrows as you type, matching name, source and provided
command names. It composes with an active facet rather than replacing it, so
"outdated things matching python" is two gestures.

`esc` clears the filter; `esc` again clears the facet. Narrowest first.

## Acceptance criteria

- [ ] `/` filters as you type, across name, source and provided commands
- [ ] The filter composes with an active facet
- [ ] `esc` clears the filter, then the facet, in that order
- [ ] The cursor lands on the first match rather than staying where it was
- [ ] A filter matching nothing says so and says what would clear it

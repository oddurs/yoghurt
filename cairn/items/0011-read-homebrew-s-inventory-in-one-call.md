---
id: 11
title: Read Homebrew's inventory in one call
type: feature
status: backlog
milestone: v0.1
depends_on:
- 8
- 9
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: source
---

## Problem

169 formulae and 10 casks, and shelling out per package would cost minutes.

## Proposal

One `brew info --json=v2 --installed` call, mapped to facts using the field
mapping the spike established. `installed_on_request` becomes `Wanted` — that
single flag is what separates the 47 you chose from the 122 you did not, and it
is the insight the whole tool rests on.

Casks are formulae as far as the graph is concerned: they emit the same facts
with a different source label.

## Acceptance criteria

- [ ] One subprocess for the whole inventory, formulae and casks
- [ ] `installed_on_request` becomes a `Wanted` fact
- [ ] The dependency list becomes `DependsOn` facts
- [ ] Sizes and install dates are emitted where Homebrew reports them
- [ ] Homebrew not being installed yields no facts and no error
- [ ] `brew` failing or returning unparseable JSON is reported, not swallowed
- [ ] Parsed against a captured JSON fixture in tests, with no network and no brew

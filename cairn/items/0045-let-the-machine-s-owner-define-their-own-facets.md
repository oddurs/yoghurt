---
id: 45
title: Let the machine's owner define their own facets
type: feature
status: backlog
milestone: v0.3
depends_on:
- 20
- 41
created: 2026-09-13
updated: 2026-09-13
priority: p2
effort: m
area: config
---

## Problem

The five facets are the ones this design anticipated. Somebody who cares about
"everything installed before I got this laptop" has no way to ask for it twice.

## Proposal

Facets are already saved queries rather than hardcoded branches, so exposing
them in `~/.config/yoghurt/config.toml` is a parser, not an architecture change.
The built-in five are defined through the same mechanism, so a built-in can
never do something a user-defined one cannot.

## Acceptance criteria

- [ ] Facets are declared in config with a name and a query
- [ ] The built-in facets are defined through the same mechanism
- [ ] An invalid query names the facet and the problem, and the rest still load
- [ ] User facets appear in the strip alongside the built-ins
- [ ] The config format is documented with a worked example

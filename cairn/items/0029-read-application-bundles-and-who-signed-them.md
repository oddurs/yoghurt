---
id: 29
title: Read application bundles and who signed them
type: feature
status: backlog
milestone: v0.2
depends_on:
- 24
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: source
---

## Problem

46 applications, 10 of which Homebrew installed as casks. The other 36 came from
somewhere, and right now that somewhere is unknowable.

## Proposal

Read `Info.plist` for bundle identifier, version and minimum system version, and
the signing authority for who shipped it. Use whichever methods the spike ranked
as affordable. An app Homebrew already claims as a cask is not reported twice —
the graph reconciles them by artifact path.

## Acceptance criteria

- [ ] Every bundle carries a name, version and bundle identifier
- [ ] The signing authority is shown where it exists
- [ ] A cask-installed app is one node, owned by Homebrew, not two
- [ ] An unsigned or damaged bundle is reported as such rather than skipped
- [ ] Reading 46 bundles costs under 200ms

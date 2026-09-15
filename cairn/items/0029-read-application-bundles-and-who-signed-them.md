---
id: 29
title: Read application bundles and who signed them
type: feature
status: done
milestone: v0.2
depends_on:
- 24
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: m
area: source
---

## Problem

Application bundles needed reading: name, version, and who shipped them.

## Superseded by 0073

Written before the scale of the problem was measured. 0073 covers everything
here and more: it reads `Info.plist` for version and bundle identifier, reads
the `codesign` authority for the vendor, and treats a Mac App Store receipt as a
*source* rather than as metadata — which this item did not anticipate and which
is the reason 10 applications stopped being orphans rather than merely being
labelled.

Closed rather than dropped: the work was done, under a different number.

## Acceptance criteria

- [x] Every bundle carries a name, version and bundle identifier
- [x] The signing authority is shown where it exists
- [x] A cask-installed app is one node, owned by Homebrew, not two
- [x] An unsigned or damaged bundle is reported as such rather than skipped
- [x] Reading 46 bundles costs under 200ms

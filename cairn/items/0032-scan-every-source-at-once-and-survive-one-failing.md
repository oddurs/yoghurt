---
id: 32
title: Scan every source at once, and survive one failing
type: feature
status: done
milestone: v0.2
depends_on:
- 31
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: m
area: runtime
---

## Problem

One slow package manager must never hold up the other seven, and one broken one
must never blank the whole view.

## Proposal

Each adapter runs on its own thread and reports facts as it finishes. The graph
is rebuilt incrementally. A source still working shows `…` for its count; a
source that failed shows its error in the strip and leaves everything else
intact.

## Acceptance criteria

- [ ] Sources scan concurrently; total time is the slowest, not the sum
- [ ] A source still scanning shows a pending state in its group header
- [ ] A failing source shows what failed, and the other sources still render
- [ ] The interface stays responsive to input throughout a scan
- [ ] A scan can be cancelled by quitting without leaving a thread behind

## 2026-09-15

Built with 0058 in one branch: concurrency is what makes partial failure possible in the first place, and a scan that runs eight sources at once but still returns the first error would be a worse answer than the sequential one it replaced.

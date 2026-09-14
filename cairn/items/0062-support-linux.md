---
id: 62
title: Support Linux
type: feature
status: backlog
milestone: later
created: 2026-09-13
updated: 2026-09-13
priority: p2
effort: l
area: source
---

## Problem

v1.0 promises macOS and nothing else. Linux doubles the audience and the code is
mostly portable already — but Linuxbrew, apt, dnf, pacman and Flatpak are five
more adapters and a different idea of what "installed" means.

## Proposal

Parked deliberately. The v1.0 promise is narrow so that it can be kept. Revisit
once the adapter contract has survived eight implementations and the shape is
known to be right.

## Acceptance criteria

- [ ] A decision on which distributions are in scope, written down
- [ ] The PATH walk and graph work unchanged on Linux
- [ ] `/Applications` handling becomes a macOS-only source rather than an assumption

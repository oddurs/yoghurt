---
id: 2
key: v0.2
title: All of it
type: milestone
status: done
created: 2026-09-13
updated: 2026-09-15
priority: p2
due: 2026-12-01
---

## Ships

Every package manager on the machine, not just Homebrew — and it opens
instantly instead of waiting for a scan.

## Done when

- [ ] cargo, rustup, go, npm/pnpm/yarn, gem, pipx and uv all appear
- [ ] Application bundles carry a version and a signing identity
- [ ] A binary nobody claims is named where it can be, and says so where it cannot
- [ ] The first frame comes from cache and says how old it is
- [ ] Every source scans concurrently; one slow manager never holds up the rest
- [ ] A source that fails leaves the other seven intact and says what failed
- [ ] The list groups by role, size and age as well as source
- [ ] Everything reads correctly under `mono` and `NO_COLOR`

## Explicitly not in this milestone

- The Path view. Shadowing needs every adapter before it means anything
- The Map view
- Watching the filesystem. `r` rescans; that is enough

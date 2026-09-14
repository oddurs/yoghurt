---
id: 1
key: v0.1
title: See it
type: milestone
status: backlog
created: 2026-09-13
updated: 2026-09-13
priority: p2
due: 2026-10-20
---

## Ships

Run `yoghurt` in a terminal and see everything Homebrew installed — what you
chose against what came with it — and everything else on the machine that
Homebrew does not account for.

## Why this is the MVP

The graph is the whole idea, and one adapter is enough to prove it. The PATH
walk is ground truth, so every binary Homebrew does not claim already shows as
an orphan on day one. That is useful immediately, on a machine where 36
applications and 122 unrequested formulae are currently invisible.

## Done when

- [ ] `yoghurt` in a terminal opens an interface; `yoghurt | cat` prints a table
- [ ] Homebrew formulae and casks appear with version, size and age
- [ ] Each row says whether it was wanted or pulled in
- [ ] Anything on PATH or in /Applications that Homebrew does not claim shows as an orphan
- [ ] The detail pane traces a dependency back to the thing that dragged it in
- [ ] Clicking a facet in the status strip filters the list
- [ ] Mouse and keyboard both drive every one of those
- [ ] The terminal is restored on exit, on panic, and on SIGTERM

## Explicitly not in this milestone

- Any package manager other than Homebrew
- The Path view and the Map view
- Caching — a two-second first scan is acceptable here
- Naming orphans. They show as orphans; identifying them comes in v0.2

---
id: 81
title: Find what is out of date, for every source
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 80
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
area: source
---

## Problem

45 packages are out of date and every one of them is Homebrew's, because
Homebrew is the only source that reports it. The other 75 — cargo, rustup,
applications, App Store — have no update detection at all, so the `outdated`
facet is really "outdated, as far as Homebrew knows".

## Proposal

Ask each source what it can. Cargo can compare against crates.io, rustup against
the channel manifests, casks and apps against what Homebrew and Sparkle already
publish.

All of it is network, so none of it runs at startup. It happens on `r`, or not
at all, and the interface distinguishes "current" from "not checked" rather than
showing them the same way — which is what it does today for 75 packages.

## Acceptance criteria

- [ ] Each source reports outdated where it can
- [ ] Nothing contacts the network unless the rescan asked for it
- [ ] The newer version is shown beside the installed one
- [ ] "Not checked" is visibly different from "current"
- [ ] A source that cannot be reached leaves its packages unchanged, not wrong

## 2026-09-15

Measured: 225 of 300 checked, 47 outdated (was 45). Found cargo-nextest 0.9.143 -> 0.9.144 and typst-cli 0.14.2 -> 0.15.1, which yoghurt was blind to. rustup reports 0/9 honestly: this machine's toolchains are version-pinned directories and rustup check only speaks in channels, so nothing can be attributed. Applications and App Store are 0 by design — no updates impl, so they read as not checked rather than falsely current.

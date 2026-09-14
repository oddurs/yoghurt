---
id: 24
title: How do you identify a binary nobody claims?
type: spike
status: backlog
milestone: v0.2
depends_on:
- 10
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: source
---

## Question

Given a binary or a bundle that no package manager claims, how much can be
recovered about where it came from?

## Why it has to be answered before the work

Orphans are the largest blind spot on the machine — 36 applications here — and
"unknown" is a much weaker answer than "you dragged this out of a dmg in 2024,
signed by JetBrains". Whether that is achievable decides whether naming orphans
is a feature or a paragraph in the README explaining why it is not.

## Options

- `go version -m` reads the module path and version straight out of a Go binary
- Mach-O load commands carry an identifier and sometimes a version
- `codesign -dv` gives the signing authority, which names the vendor
- `Info.plist` gives bundle id, version and minimum system for an app
- Extended attributes record the download URL (`com.apple.metadata:kMDItemWhereFroms`)

## What would settle it

Run each against a sample of twenty real orphans on this machine and count how
many get a usable answer from each method. Note the cost per binary — anything
that needs a subprocess per file is too slow for 592 of them.

Timebox: one day. The answer is a ranked list of methods with hit rates and costs.

## Answer

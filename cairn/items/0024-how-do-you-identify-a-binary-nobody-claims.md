---
id: 24
title: How do you identify a binary nobody claims?
type: spike
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 10
created: 2026-09-13
updated: 2026-09-15
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

**Yes, for 43 of 44.** Measured on this machine before 0073 was written:

| signal | apps | what it proves |
|---|---|---|
| `Contents/_MASReceipt` | 10 | the App Store installed it — a package manager |
| `codesign` authority | 32 | the vendor that shipped it, by name |
| Apple's own signature | 1 | part of the system |
| nothing | 1 | genuinely anonymous |

`pkgutil --pkgs` additionally lists 105 installer receipts, each with an install
date and a file list, so a `.pkg` install is attributable.

Ranked by cost as well as hit rate: the receipt is a `stat`, `codesign` is one
subprocess **per bundle** and must never be run per file — 45 bundles is 0.8
seconds and 3000 artifacts would be a minute.

`kMDItemWhereFroms` was absent on every application tested and is not worth
relying on.

`go version -m` reads the module path out of a Go binary and is the equivalent
trick for that ecosystem; it is used in 0028 rather than here.

Implemented in 0073, which took unclaimed from 154 to 72 and left no application
among them.

---
id: 24
title: How do you identify a binary nobody claims?
type: spike
status: backlog
milestone: v0.2
depends_on:
- 10
created: 2026-09-13
updated: 2026-09-14
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

## 2026-09-14

Measured on this machine, ahead of the spike: 44 apps in /Applications — 10 carry Contents/_MASReceipt (Mac App Store), 32 are Developer ID signed with the vendor named in the codesign authority (Figma Inc., Running with Crayons Ltd, Mitchell Hashimoto), 1 is signed by Apple, and exactly 1 is anonymous. pkgutil --pkgs additionally lists 105 installer receipts, each with an install date and file list. So 43 of 44 unclaimed apps are identifiable and 'unclaimed' is yoghurt's ignorance rather than the machine's. kMDItemWhereFroms was absent on every app tested and is not worth relying on.

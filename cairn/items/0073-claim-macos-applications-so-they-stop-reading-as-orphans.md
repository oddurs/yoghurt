---
id: 73
title: Claim macOS applications so they stop reading as orphans
type: feature
status: done
milestone: v0.2
depends_on:
- 24
created: 2026-09-14
updated: 2026-09-14
priority: p0
effort: m
area: source
---

## Problem

44 applications show as unclaimed, which says yoghurt does not know where they
came from. The machine does know. Measured: 10 carry a Mac App Store receipt,
32 are Developer ID signed with the vendor named in the signature, 1 is Apple's
own, and exactly **one** is genuinely anonymous.

Showing 43 identifiable applications as orphans is the same mistake the
Homebrew adapter nearly made with untrusted taps: treating a gap in what we
asked as a gap in what exists.

## Proposal

A macOS source that claims what it can, using three signals in order of
confidence:

1. **`Contents/_MASReceipt`** — the App Store installed it. That is a package
   manager and it should appear as one, not as an orphan.
2. **`codesign` authority** — `Developer ID Application: Figma, Inc.` names the
   vendor. Not an installer, so it does not make the app *owned*, but it is a
   provenance fact worth carrying.
3. **`pkgutil --pkgs`** — 105 installer receipts on this machine, each with an
   install date and a file list, so a `.pkg` install can be attributed.

An app that none of the three explain stays unclaimed, honestly, and on this
machine that is one app.

## Acceptance criteria

- [ ] App Store applications appear under an `app store` source, not as orphans
- [ ] A Developer ID signature yields the vendor name as a fact
- [ ] `pkgutil` receipts attribute the files they installed
- [ ] An application none of these explain is still reported as unclaimed
- [ ] `codesign` is not run per file on the whole machine; only on bundles
- [ ] Tested against a fixture tree, with the signing data injected

## 2026-09-14

Unclaimed dropped 154 -> 72 and not one application is left among them. Six sources now: homebrew 224, applications 35, cargo 21, app store 10, rustup 9. codesign runs once per bundle and costs 0.8s for 45; running it per artifact would have cost a minute.

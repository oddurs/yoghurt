---
id: 78
title: Read what Homebrew already says a package is for
type: feature
status: done
milestone: v0.2
created: 2026-09-14
updated: 2026-09-14
priority: p1
effort: s
area: source
---

## Problem

Every one of the 210 formulae carries a `desc` — "Codec library for encoding and
decoding AV1 video streams", "Improved shell history for zsh, bash, fish and
nushell" — and yoghurt reads none of them. The detail pane has nothing to say
about what a thing is *for*, and `/` cannot find a package by what it does.

## Proposal

A `Describes` fact carrying the sentence, emitted by any adapter that has one.
Homebrew has it for everything; npm has `description`; cargo has it in
`.crates2.json`.

Shown in the detail pane, and searched by `/`, so "video" finds the codecs.

## Acceptance criteria

- [ ] `desc` becomes a fact wherever a source reports one
- [ ] The detail pane shows it
- [ ] `/` matches against it
- [ ] A source with no description is not an error

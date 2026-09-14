---
id: 54
title: Build release binaries on a tag
type: chore
status: backlog
milestone: v1.0
depends_on:
- 53
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: packaging
---

## Problem

The release workflow runs checks and generates notes, but ships no binary.
Anyone without a Rust toolchain cannot run this.

## Proposal

A tagged push builds for Apple silicon and Intel, attaches both to the release,
and updates the tap. Notes stay generated and stay stripped of any assistant
attribution, as the existing workflow already does.

## Acceptance criteria

- [ ] A `v*` tag produces signed binaries for both architectures
- [ ] Checksums are attached and verifiable
- [ ] The tap formula updates automatically from the released artefacts
- [ ] A dry run is possible without publishing
- [ ] Release notes contain no attribution, asserted by the existing filter

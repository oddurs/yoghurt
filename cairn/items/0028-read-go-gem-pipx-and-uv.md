---
id: 28
title: Read go, gem, pipx and uv
type: feature
status: backlog
milestone: v0.2
depends_on:
- 9
- 24
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: source
---

## Problem

Four more managers, each small, none worth its own item.

## Proposal

Go binaries are the interesting case: `go install` leaves no manifest, but the
module path and version are embedded in the binary and readable with
`go version -m`. The spike settles whether that is affordable per binary.

Gem reads its own directories. Pipx and uv each keep a venv per tool with a
metadata file.

## Acceptance criteria

- [ ] Go binaries in `$GOBIN` or `$GOPATH/bin` carry module path and version
- [ ] Gems appear with version and the executables they install
- [ ] pipx and uv tools appear with their Python version and injected packages
- [ ] Each adapter is under a hundred lines and touches nothing outside itself
- [ ] Each is tested against a fixture tree

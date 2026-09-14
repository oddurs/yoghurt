---
id: 27
title: Read global npm, pnpm and yarn packages
type: feature
status: backlog
milestone: v0.2
depends_on:
- 9
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: source
---

## Problem

Global node packages hide in a different place per package manager, per
installation method, and per version manager. There is no single answer to
"where are they".

## Proposal

Look in the known roots rather than shelling out: Homebrew's `lib/node_modules`,
`/usr/local/lib/node_modules`, `~/.npm-global`, and the version-manager
directories for fnm, nvm and volta. Read each package's own `package.json` for
name, version and its `bin` map — that map is what makes `Provides` facts
correct rather than guessed.

Scoped packages live one directory deeper and are easy to miss.

## Acceptance criteria

- [ ] Packages found under every listed root, deduplicated by realpath
- [ ] Scoped packages are found and named `@scope/name`
- [ ] The `bin` map becomes `Provides` facts
- [ ] Packages installed under a version manager say which runtime version owns them
- [ ] A malformed `package.json` is skipped with a warning, not a panic

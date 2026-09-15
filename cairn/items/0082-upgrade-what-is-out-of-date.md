---
id: 82
title: Upgrade what is out of date
type: feature
status: backlog
milestone: v0.3
depends_on:
- 81
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
area: cli
---

## Problem

yoghurt can tell you 45 things are out of date and then leaves you to go and
type the upgrades somewhere else, re-deriving which manager owns each one — the
join it already knows.

## Proposal

The first thing yoghurt does that changes the machine, so it sets the pattern
for everything after it.

`u` on a row composes the upgrade for that package, using the source that owns
it: `brew upgrade pandoc`, `cargo install --force ripgrep`. The command is shown
verbatim. Nothing runs until a confirmation is typed, and the default is to do
nothing.

On a marked set it composes **one** command per source rather than one per row.

Output is streamed rather than swallowed, because an upgrade that fails halfway
has to be readable.

## Acceptance criteria

- [ ] `u` composes the right command for the source that owns the row
- [ ] The command is shown verbatim before anything happens
- [ ] Confirmation is typed; a keystroke is never enough
- [ ] Refusing leaves the machine untouched and says so
- [ ] Output is streamed, and a failure is reported with its exit code
- [ ] A source with no upgrade command says so rather than guessing one
- [ ] The interface rescans afterwards, so what it shows is what is now true

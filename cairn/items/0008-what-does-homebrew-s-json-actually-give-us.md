---
id: 8
title: What does Homebrew's JSON actually give us?
type: spike
status: backlog
milestone: v0.1
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: source
---

## Question

Does `brew info --json=v2 --installed` carry everything the graph needs, in one
call, for both formulae and casks?

## Why it has to be answered before the work

The whole architecture rests on one subprocess per source rather than one per
package. If sizes are missing, or `installed_on_request` is absent for casks,
or keg-only formulae report differently, the Homebrew adapter is a different
shape and so is the `Fact` vocabulary it emits.

## Options

- One `--json=v2 --installed` call carries everything
- It carries most of it, and sizes come from walking the Cellar
- Casks need a separate call

## What would settle it

Run it on this machine, which has 169 formulae and 10 casks, and check for each
of: name, installed version, `installed_on_request`, dependency list, size on
disk, install date, keg-only status, linked status, and the binaries the
formula provides.

Timebox: half a day. Write the field mapping into the answer.

## Answer

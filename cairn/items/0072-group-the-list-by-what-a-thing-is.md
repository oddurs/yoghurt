---
id: 72
title: Group the list by what a thing is
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p1
effort: s
area: list
---

## Problem

The axes say where something came from, whether you chose it, how big it is and
what is wrong with it. None of them say *what it is*. On this machine 70 of 224
packages put nothing on the path at all — they are libraries something else
links against — and nothing distinguishes them from the 154 you can actually
run.

## Proposal

A `category` axis, derived from the graph rather than from a list somebody has
to maintain:

    application   owns a .app bundle
    tool          puts at least one command on your path
    library       owns something, puts nothing on your path
    unclaimed     nothing owns it

Every one of those is a question about edges, which is the same rule the four
structural questions follow. No curated mapping, so a package manager this
program has never heard of is categorised correctly the moment its adapter
lands.

Fonts and language runtimes are deliberately absent: both would need either a
name-prefix heuristic or artifact parsing that does not exist yet.

## Acceptance criteria

- [ ] `g` reaches a `category` axis
- [ ] Categories are derived from ownership and provided commands, not a list
- [ ] An application bundle is an application whoever installed it
- [ ] A package providing no command is a library
- [ ] Each category is tested against a hand-built graph

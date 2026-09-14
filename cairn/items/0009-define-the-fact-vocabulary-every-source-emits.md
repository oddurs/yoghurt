---
id: 9
title: Define the fact vocabulary every source emits
type: feature
status: backlog
milestone: v0.1
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: source
---

## Problem

If adapters return a rich `Package` struct, every adapter has to know what the
interface wants, and every new field means touching all of them. The coupling
runs the wrong way.

## Proposal

An adapter's entire job is `fn scan(&self) -> Result<Vec<Fact>>`. `Fact` is a
small enum and nothing else crosses the boundary:

    Owns { package, artifact }
    Provides { artifact, command }
    DependsOn { package, on }
    Wanted { package }
    Version { package, version }
    Size { artifact, bytes }
    InstalledAt { package, when }

An adapter knows nothing about the interface, nothing about other adapters, and
nothing about scoring. Facts are additive and order-independent, so two sources
disagreeing is representable rather than a panic.

## Acceptance criteria

- [ ] `Fact` is defined with a doc comment per variant saying what it asserts
- [ ] The `Source` trait is the one method above and carries no other knowledge
- [ ] A fixture directory in, a known `Vec<Fact>` out — tested without a real machine
- [ ] Emitting the same fact twice is not an error

---
id: 9
title: Define the fact vocabulary every source emits
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-13
updated: 2026-09-14
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

## 2026-09-14

Source has two methods, not one: name() as well as scan(). Identity rather than knowledge — a partial scan has to be able to say which source failed (0058), and a dyn Source cannot carry an associated const. Vocabulary came out at nine variants rather than seven: added Outdated (brew reports it, the design shows an arrow for it) and SearchPath, which the walk emits so that contested commands are resolvable at all. Version folded into Package rather than standing alone, so one fact declares a package exists.

## 2026-09-14

Renamed the bootstrap-era Source struct to SourceSummary so the trait could take the good name. SourceSummary and survey() are superseded by the graph in 0021.

## 2026-09-14

Renamed ScanError.source to source_name in self-review: a field called  on a type implementing Error reads as Error::source(), which means the cause, not the origin.

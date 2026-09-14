---
id: 11
title: Read Homebrew from the Cellar, enriched by the JSON
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 8
- 9
created: 2026-09-13
updated: 2026-09-14
priority: p0
effort: l
area: source
---

## Problem

213 formulae and 11 casks, and shelling out per package would cost minutes. But
one JSON call is not an inventory either: spike 0008 found it silently omits
packages from untrusted taps, carries no sizes, and never names the binaries a
formula provides.

## Proposal

The Cellar is the inventory; the JSON enriches it.

Walk `$(brew --prefix)/Cellar/<name>/<version>` for what exists, and read one
`brew info --json=v2 --installed` for what Homebrew knows about it. A keg with
no JSON entry is still a package with fewer facts — never a missing one, and
never an orphan, because Homebrew does own it.

`installed_on_request` becomes `Wanted`, and that single flag is what separates
the 77 you chose from the 133 you did not. It is the insight the whole tool
rests on, and it is per keg: 44 formulae here have more than one installed.

The adapter does **not** emit `Provides`. `/opt/homebrew/bin/rg` is a symlink
into the keg, so the `PATH` walk produces that fact once it resolves symlinks,
and the graph joins the two by path prefix.

Casks are a separate shape, not formulae with a different label: different field
names throughout, `Wanted` unconditionally, and an `artifacts` list that is only
sometimes an app.

## Acceptance criteria

- [ ] Every keg under `Cellar/*/*` becomes a package, whether or not the JSON knows it
- [ ] A formula from an untrusted tap appears, owned by Homebrew, not as an orphan
- [ ] `installed_on_request` becomes a `Wanted` fact, read per keg rather than `installed[0]`
- [ ] A formula with two installed kegs yields two versions and two sizes, not one
- [ ] `runtime_dependencies` become `DependsOn` facts carrying `declared_directly`
- [ ] Size comes from the keg directory `Cellar/<name>/<version>`, and every package has one
- [ ] Casks are read with their own field names and are always `Wanted`
- [ ] The adapter emits no `Provides` facts; that is the walk's job
- [ ] Homebrew not being installed yields no facts and no error
- [ ] `brew` failing or returning unparseable JSON is reported, not swallowed
- [ ] Tested against a fixture Cellar and a captured JSON file — no network, no brew —
      and the fixture contains a cask, a multi-keg formula and an untrusted-tap keg

## 2026-09-14

The walk resolves symlinks with canonicalize(), so ownership matching needs brew's keg paths to be canonical too. If $(brew --prefix) or any ancestor is itself a symlink, keg paths as brew reports them will not prefix-match the walk's resolved paths and every linked binary becomes an orphan. Canonicalize the Cellar root once in the adapter.

## 2026-09-14

Measured on the real machine: 224 packages, 88 wanted, 136 pulled in, every one with a size; the three untrusted-tap formulae appear correctly. Adapter scan is 930ms — the Cellar size walk is 1.8s over 160,000 entries and the brew subprocess is 1.4s, so the subprocess is spawned at construction and collected after the disk has been measured. In sequence it was 1.9s.

## 2026-09-14

One package per formula, not one per keg. Two kegs of fmt are one package that owns two artifacts with their own sizes; the version shown is the linked keg, because that is the one you get when you run it. Splitting into fmt@12.1.0 and fmt@12.2.0 would have made dependency edges unresolvable, since runtime_dependencies names a formula.

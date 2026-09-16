---
id: 83
title: Versions sort as text rather than as numbers
type: bug
status: done
milestone: v0.3
assignee: Oddur Sigurdsson
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
area: list
---

## What happens

Sorting by version puts `1.12.4` before `1.3.1`, `10.4.0` before `2.16.0`, and
`126.1.2` before `2.16.0`. The whole column is ordered as text.

## What should happen

`1.3.1` before `1.12.4`, and `9.x` before `10.x`.

## Reproduction

    yoghurt --screenshot 64x300 --group source --sort version

Reads: `1.0.0 1.12.4 1.3.1 1.30.0 1.4.4 1.569.0 1.9.1 1.9.14 10.4.0 12.8 126.1.2 154.0 2.16.0`

## Fix

Compare segment by segment, numerically where both segments are numbers and as
text otherwise — so `1.10` beats `1.9` and `1.0-beta` still orders sensibly
against `1.0`. Not a semver parser: versions here come from eight ecosystems and
several of them are not semver at all.

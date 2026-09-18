---
id: 88
title: Say 'system' and 'wanted' instead of 'unexplained'
type: bug
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
created: 2026-09-17
updated: 2026-09-17
priority: p1
area: graph
effort: m
---

## What happens

## What should happen

## Reproduction

1.

## 2026-09-17

`unexplained` covers two opposite situations on this machine, and misleads in
both directions.

Ten rows are Ruby's default gems under `/Library/Ruby/Gems/2.6.0`, owned by
root, shipped with macOS. Nobody installed them and nobody can remove them.
Calling them unexplained invites a hunt for a cause that does not exist.

Four rows are `opencode`, `quarry`, `stripe`, `supabase` — formulae the owner
deliberately installed. They lack a `Wanted` fact only because they come from
untrusted taps, which `brew info --json` omits. The word says "nothing wants
this" about four things that were asked for by name.

Split it. `system` for what the OS shipped, `wanted` for a package that is
present in the Cellar with an install receipt naming it as requested. Keep
`unexplained` for what is genuinely neither, and expect it to be near empty.

## 2026-09-17

Fixed. The Cellar's own INSTALL_RECEIPT.json now decides wanted, which works for untrusted taps because every keg has one — all 214 on this machine. Gem roots carry whether the OS shipped them, so /Library/Ruby's default gems report System. Package.wanted and Package.system collapsed into one Origin field: clippy caught four bools, and the pair could represent a package both asked for and shipped, which cannot happen. On this machine unexplained went 14 to 0.

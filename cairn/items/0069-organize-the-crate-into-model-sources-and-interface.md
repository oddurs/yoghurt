---
id: 69
title: Organize the crate into model, sources and interface
type: chore
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p1
effort: s
area: runtime
---

## Problem

`src/` is flat and about to gain four more files for the interface. The design
has a clear shape — sources emit facts, the graph holds all logic, views project
it — and the directory does not show it. A reader opening the crate cannot tell
which of seven files is the model and which is an adapter.

## Proposal

Three directories that name the three layers, in the order data moves through
them:

    src/model/     fact, graph, question    what the machine is
    src/source/    walk, homebrew           where facts come from
    src/view/      plain, and the TUI       how it is shown

`main.rs` keeps argument parsing and nothing else. The table moves out of it
into `view/plain.rs`, so the interface added next sits beside it rather than
inside `main`.

Re-exports at the crate root stay, so this is not a breaking rearrangement for
anything outside.

## Acceptance criteria

- [ ] Every module lives under `model`, `source` or `view`
- [ ] `main.rs` contains argument parsing and the entry point, nothing else
- [ ] The crate root still re-exports `Fact`, `Graph`, `Source` and the adapters
- [ ] No behaviour changes; the test count is unchanged or higher
- [ ] Each directory's `mod.rs` says in a sentence what belongs in it

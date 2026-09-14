---
id: 8
title: What does Homebrew's JSON actually give us?
type: spike
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
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

**Yes, with one gap: size.** One `brew info --json=v2 --installed` call returns
1.0 MB covering 210 formulae and 11 casks in 1.3 seconds. The architecture holds
— one subprocess per source, not one per package.

### Field mapping for formulae

| Fact | Where it comes from |
|---|---|
| `Wanted` | `installed[0].installed_on_request` |
| `DependsOn` | `installed[0].runtime_dependencies[].full_name` |
| `Version` | `installed[0].version` |
| `InstalledAt` | `installed[0].time` — Unix seconds |
| `Size` | **not present.** Walk `$(brew --prefix)/Cellar/<name>` |

`runtime_dependencies` entries also carry `declared_directly`, which
distinguishes a dependency the formula asked for from one it inherited. Worth
keeping: it makes the why-chain shorter and more honest.

Also usable, though not yet facts: `outdated` (40 of 210 here), `keg_only`,
`linked_keg`, `pinned`, `deprecated` and `disabled`.

### Casks are different in two ways

1. **No `installed_on_request`.** There is no such thing as a cask installed as
   somebody else's dependency, so every cask is `Wanted` unconditionally.
2. **The field names differ**: `token` not `name`, `installed` is the version
   string itself, and the timestamp is `installed_time` at the top level rather
   than inside an `installed[]` array.

`artifacts` says what a cask actually put on disk, and it is not always an app —
of 11 casks here, 4 install a `.app`, the rest install binaries, fonts or a
`.pkg`. The shape is a nested list: `{"app": [["Docker.app"]]}`. So the cask →
`/Applications` link is real but partial, and the adapter must handle `app`,
`binary`, `font` and `pkg` artifacts rather than assuming an app.

### Sizes

`du` on the keg directory matches what Homebrew reports elsewhere — `ripgrep` is
6.5 MB by both. Walking 213 keg directories is one `stat` pass and is cheap
enough to do inline. Casks have no keg, so a cask's size is the size of the
artifact it installed, which is why the artifact list matters.

### A correction to the numbers this project has been quoting

The design doc and the roadmap cite 169 formulae and 47 leaves. Measured again:

    brew list --formula     213
    brew leaves              73
    JSON formulae           210    installed_on_request  77

The 210/213 difference is casks-with-binaries counting differently between the
two commands. The important one is that **`brew leaves` and
`installed_on_request` are not the same question** and this project needs the
second: `leaves` means "nothing depends on it", which hides a package you
installed deliberately that something later depended on. `installed_on_request`
means "you typed this", which is what the interface claims to show.

So the real figure is 77 of 210 requested — 63% of Homebrew arrived as a
consequence rather than a decision. The story is unchanged; the numbers in the
docs are wrong and are corrected in 0023.

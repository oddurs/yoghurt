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

Run it on this machine and check for each
of: name, installed version, `installed_on_request`, dependency list, size on
disk, install date, keg-only status, linked status, and the binaries the
formula provides.

Timebox: half a day. Write the field mapping into the answer.

## Answer

**No — it carries most of it, and three things it does not carry are
load-bearing.** One `brew info --json=v2 --installed` call returns 1.0 MB
covering 210 formulae and 11 casks in 1.3 seconds, so the one-subprocess shape
holds. But it is an enrichment source, not an inventory: the Cellar is the
inventory.

### What it does carry, for formulae

| Fact | Where it comes from |
|---|---|
| `Wanted` | `installed[n].installed_on_request` |
| `DependsOn` | `installed[n].runtime_dependencies[].full_name` |
| `Version` | `installed[n].version` |
| `InstalledAt` | `installed[n].time` — Unix seconds |

`runtime_dependencies` entries also carry `declared_directly`, distinguishing a
dependency the formula asked for from one it inherited. That keeps a why-chain
short. Also usable: `outdated` (40 of 210 here), `keg_only`, `linked_keg`,
`pinned`, `deprecated`, `disabled`.

### The three gaps

**1. It does not name the binaries a formula provides.** There is no `bin` key.
`aliases` is a formula alias, not an executable. This turns out not to matter,
and the reason is worth recording: `/opt/homebrew/bin/rg` is a symlink to
`../Cellar/ripgrep/15.2.0/bin/rg`, so the `PATH` walk in 0010 already produces
the `Provides` fact once it resolves symlinks. The Homebrew adapter emits
`Owns` for the keg directory and the graph joins the two **by path prefix** —
the resolved command lives *under* the owned keg. So `Provides` is entirely the
walk's job, and the adapter is simpler than planned.

That prefix join is a requirement on 0012 and 0013: ownership cannot be matched
by path equality.

**2. It does not carry sizes.** Sizes come from `du` on the keg — and the path
is `Cellar/<name>/<version>`, not `Cellar/<name>`. **44 of the 210 formulae
here have more than one installed keg**, so measuring the parent directory
would overstate a fifth of the machine. `installed[0]` is wrong for the same
reason: a formula with two kegs has two versions, two install dates and
potentially two answers to `installed_on_request`.

**3. It silently omits packages from untrusted taps.** `brew list --formula`
reports 213; the JSON reports 210. The missing three are `opencode`, `stripe`
and `supabase`, all from third-party taps, and `brew info --json` on any of
them fails with *"Refusing to load formula from untrusted tap"*. The
`--installed` dump does not error and does not mention them — they are simply
absent, while remaining installed, linked into `bin`, and on the user's `PATH`.

This is the important finding. **A Homebrew adapter built on the JSON alone
would silently lose every package from an untrusted tap**, and those would then
surface as orphans — which is exactly the wrong answer, because Homebrew does
own them.

So the adapter walks `Cellar/*/` for the inventory and uses the JSON to enrich
it. A keg with no JSON entry is still a package; it just has fewer facts. This
is the same principle the design already applies to `PATH`: the filesystem is
ground truth, and a manager's claims are checked against it.

### Casks diverge more than expected

1. **No `installed_on_request`.** A cask is never somebody else's dependency, so
   every cask is `Wanted` unconditionally.
2. **Different field names throughout**: `token` not `name`, `installed` is the
   version string itself, and the timestamp is `installed_time` at the top
   level rather than inside an `installed[]` array.
3. **`artifacts` is not always an app.** Of 11 casks here, 4 install a `.app`;
   the rest install binaries, fonts or a `.pkg`. The shape is a nested list:
   `{"app": [["Docker.app"]]}`.
4. **Cask size is not solved.** An `app` artifact can be measured. A `binary`
   artifact is a symlink into the Caskroom, so following it double-counts and
   not following it reports nothing. A `pkg` artifact leaves nothing
   attributable on disk at all. Since 7 of 11 casks here are not apps, this is
   the majority case and it stays open.

### The numbers this project has been quoting are wrong

    brew list --formula     213
    brew leaves              73
    JSON formulae           210    installed_on_request  77

The docs say 169 and 47. Beyond being stale, **`brew leaves` answers a different
question from `installed_on_request`**, and this project needs the second:
`leaves` means "nothing depends on it", which hides a package you installed
deliberately that something later depended on. `installed_on_request` means
"you typed this".

The real figure is 77 of 210 requested — 63% of Homebrew arrived as a
consequence rather than a decision. The story is unchanged; the figures are
corrected in `docs/interface.md` under 0068.

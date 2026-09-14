# The interface

What yoghurt shows, and how. Written before the interface exists, from a real
machine's numbers, so the design argues with data rather than with taste.

## The machine this was designed against

```
brew formulae      213      requested           77
brew casks          11      /Applications       46
cargo install       22      binaries on PATH   592
rustup toolchains    7      homebrew on disk   5.6G
gem                 48      largest keg        1.5G  (llvm@21)
npm global           3      multi-keg formulae  44
```

Three numbers in there are the whole design.

**213 formulae, 77 requested.** Sixty-three percent of what is installed, nobody
asked for. A flat list of 213 rows treats `ripgrep` and `libunistring` as peers,
and they are not: one is a decision, the other is a consequence. The interface
has to separate what you *wanted* from what came *with*.

The measure is Homebrew's own `installed_on_request` — "you typed this" — and
not `brew leaves`, which asks whether anything currently depends on it. Those
are different questions, and `leaves` hides a package you chose deliberately
that something later came to depend on.

**46 applications, 11 casks.** Thirty-five applications that no package manager
claims. Dragged from a `.dmg`, installed by a vendor updater, left behind by
something uninstalled years ago. This is the "custom stuff", and it is the
largest single blind spot on the machine.

It is not only the things no manager installed. Three of those 213 formulae come
from untrusted taps, and Homebrew's own JSON silently refuses to report them —
installed, linked, on the `PATH`, and invisible to the tool that installed them.
A survey built on what each manager says about itself would lose them.

**592 binaries on one PATH entry alone.** The number of *commands* is far larger
than the number of *packages*, and the mapping between them is invisible. When
two packages both provide `python3`, nothing tells you which one wins.

So: a provenance gap, an intent gap, and a resolution gap.

## The model

Those three gaps are a way of noticing the problem, not a reason for three
views. The reason is that a machine has exactly three kinds of thing worth
naming, and everything else is an edge between them.

    package   a manager's record of something      -> Inventory
    artifact  actual bytes on disk                 -> Map
    command   a name resolvable on PATH            -> Path

    package  --owns-->     artifact
    artifact --provides--> command
    package  --depends-->  package
    (you)    --wanted-->   package

Every state this interface shows is then a query over that graph rather than a
feature somebody remembered to build:

    orphan     an artifact with no owning package
    broken     a package owning an artifact that is not there
    pulled in  a package reachable only through `depends`, never from `wanted`
    shadowed   two artifacts providing one command; PATH order decides

Four structural facts generate every glyph below. That is the test the design
has to pass: the model produces the features instead of listing them. It also
says there is no fourth view, because there is no fourth kind of node.

Architecturally this falls out as one direction of dependency:

    adapters --emit--> facts --assemble--> graph --project--> views

An adapter's entire job is `fn scan(&self) -> Vec<Fact>`. It knows nothing about
the interface, nothing about the other adapters, and nothing about scoring. All
logic lives in the graph, so adding a package manager is one file that touches
nothing else.

The one thing that is not an adapter is the `PATH` walk. That is ground truth:
adapters make claims, and the walk says what is actually there. Orphans are not
detected — they are what is left over. Which is why the first version is useful
with only Homebrew wired up: everything else on the machine already shows as
`?`.

## Chrome

Two fixed lines at the top, one at the bottom. The shape is
[harrow](https://github.com/oddurs/harrow)'s, because it works and because the
two tools should feel like the same hand.

```
 yoghurt  mba-oddur   293 packages · 8 sources · 6.4G          scanned 2m ago ⟳
───────────────────────────────────────────────────────────────────────────────
 47 wanted   122 pulled in   31 outdated   4 shadowed   2 broken
```

Line one is identity and totals. The `⟳` spins while a scan is in flight, and
the timestamp is how you know whether you are looking at the machine or at a
memory of it.

Line two is the only summary, and **every facet on it is a button**. Clicking
`31 outdated` filters the inventory to those thirty-one. Clicking it again
clears. This is the single most important mouse decision in the tool: the
summary *is* the navigation, so the common case — "show me the problems" —
costs one click from anywhere, in any view.

The footer is contextual keys, changing per view.

## View 1 — Inventory

The workhorse. `tab` cycles views; the tab names in the header are clickable.

```
╭ Inventory · by source ───────────────────────────╮╭ glib ──────────────────────╮
│ ▾ homebrew           169 formulae · 4.9G      ▰▰▰││ ◐ 2.84.1 · brew · 155M     │
│   ● ripgrep      14.1.1  brew  leaf   5.2M  3mo  ││   pulled in — you did not  │
│   ● fd            10.2.0 brew  leaf   2.1M  3mo  ││   ask for this             │
│   ↑ neovim        0.11.2 brew  leaf    38M  1y  ↑││                            │
│   ◐ glib          2.84.1 brew  dep     155M 10mo││ WHY                        │
│   ◐ libunistring  1.3    brew  dep     1.8M 10mo││   glib                     │
│ ▾ applications        46 bundles · 18G       ▰▰▰ ││   └ gtk+3                  │
│   ? Xcode.app     16.4   —     orphan  14G  2y   ││     └ inkscape ← you       │
│   ● Ghostty.app   1.2.0  cask  leaf    98M  4mo  ││       installed this       │
│ ▾ cargo               22 binaries · 412M     ▰   ││                            │
│   ⊘ rg            14.1.0 cargo leaf    6.1M 1y   ││ PROVIDES                   │
│   ● cargo-nextest 0.9.9  cargo leaf     11M 2mo  ││   gio  gdbus  gsettings    │
╰──────────────────────────────────────────────────╯╰────────────────────────────╯
 ↑↓ move  ↵ inspect  space mark  g group  s sort  / filter  ! problems  ? help
```

### The row

Everything right of the name is fixed width and right-aligned, so the eye runs
down a column instead of hunting along each row.

```
 ● ripgrep                    14.1.1   brew   leaf    5.2M   3mo   ↑
 │ │                          │        │      │       │      │     └ update waiting
 │ │                          │        │      │       │      └ installed when
 │ │                          │        │      │       └ on disk
 │ │                          │        │      └ why it is here
 │ │                          │        └ which manager owns it
 │ │                          └ version
 │ └ name — the only flexible column
 └ state
```

When the pane narrows, columns drop in order of how little they answer: **age,
then role, then size, then version**. Name, state glyph and source never drop.
Below 96 columns the detail pane stops paying for itself and gets out of the
way; `↵` opens detail as a full-screen overlay instead of splitting a pane that
is already too narrow.

### State glyphs

One glyph per state, so the screen still says everything it needs to with no
colour at all — `mono`, `NO_COLOR`, or a reader who cannot tell the green from
the red.

| | State | Means |
|---|---|---|
| `●` | fine | Installed, current, claimed by a manager |
| `↑` | outdated | A newer version is published |
| `◐` | pulled in | A dependency. You did not ask for it |
| `?` | orphan | On disk or on PATH, no manager claims it |
| `⊘` | shadowed | Something else with this name wins on PATH |
| `·` | stale | Not executed in months — *parked; see below* |
| `✕` | broken | Dangling symlink, or the binary is gone |

### Grouping

`g` cycles the axis; the axis name in the pane title is clickable. Group headers
collapse on click and carry their own totals and a share-of-disk bar.

| Axis | Answers |
|---|---|
| **source** *(default)* | "What is Homebrew responsible for?" |
| **role** | "What did I actually choose?" — the 47 versus the 122 |
| **size** | ">100M, 10–100M, 1–10M, <1M" — where the 5.6G went |
| **age** | "What have I not touched in a year?" |
| **health** | Outdated, shadowed, broken, fine |

### Detail — "why is this here?"

For a leaf, detail is facts. For a dependency, it answers the question the flat
list cannot: **who dragged you in**. Walk the reverse dependency graph to the
nearest thing the user installed on purpose, and print the chain:

```
 glib
 └ gtk+3
   └ inkscape ← you installed this
```

`brew info --json=v2 --installed` carries `installed_on_request`, which is
exactly the leaf flag, and the full dependency graph, in one subprocess for all
179 packages. The chain costs nothing extra.

Detail ends in real buttons — `[ Uninstall ]  [ Pin ]  [ Copy name ]  [ Homepage ]` —
as hit rectangles, not text. Keys mirror every one of them.

## View 2 — Map

The chart. A squarified treemap where area is bytes on disk.

```
╭ Map · homebrew ▸ ────────────────────────────────────── area: size  ⌄ ──────────╮
│╭─────────────────────────╮╭──────────────╮╭─────────╮╭────────╮╭──────┬────────╮│
││                         ││              ││         ││        ││ zig  │awscli  ││
││   llvm@21               ││  openjdk     ││ pandoc  ││  go    ││ 246M │197M    ││
││   1.5G                  ││  380M        ││ 263M    ││  258M  │├──────┼───┬────┤│
││                         ││              ││         ││        ││glib  │gs │…   ││
│╰─────────────────────────╯╰──────────────╯╰─────────╯╰────────╯╰──────┴───┴────╯│
╰──────────────────────────────────────────────────────────────────────────────────╯
 llvm@21 · homebrew · 1.5G · 27% of homebrew · pulled in by swift-format
```

Cells fill with `▀` half-blocks, which doubles vertical resolution: a one-row
terminal cell is two logical rows, so small packages stay visible instead of
collapsing to nothing. Hue is the source; lightness carries age, so a wall of
dim cells reads as "old" without a legend.

Mouse is the entire point of this view:

| | |
|---|---|
| **hover** | Cell brightens; the strip line below names it, sizes it, and says who pulled it in |
| **click** | Zoom into the cell. A breadcrumb appears in the title — `Map · homebrew ▸ llvm@21` |
| **breadcrumb** | Click any segment to zoom back out |
| **drag** | Marquee. Every cell inside is marked, and the strip totals them: `7 marked · 2.1G` |
| **right-click** | Context menu: inspect, uninstall, pin, copy, reveal |
| **scroll** | Cycle what area means |

`m` cycles the metric, because "big" has three useful meanings:

- **size** — bytes. What to delete.
- **count** — packages. Which manager sprawled.
- **dependents** — how many things need it. A big cell here is load-bearing:
  delete it and a dozen things break. It is the inverse of the size map, and it
  is the one that stops you making a mistake.

Below eighty columns, or under `mono`, the treemap degrades to a sorted
horizontal bar chart with the same hit rectangles and the same interactions.
Never a blank pane.

## View 3 — Path

592 binaries, and the only question worth asking about them is which one wins.
This view answers it, and by default shows **only the contested names** — the
handful where more than one thing provides the same command. Six hundred rows
collapse to perhaps fifteen.

```
╭ Path · 14 contested of 592 ──────────────────────────────── a: show all ────────╮
│ COMMAND     WINS                             ALSO PROVIDED BY                   │
│ python3     /opt/homebrew/bin   brew  3.13   ⊘ /usr/bin        system  3.9      │
│ node        ~/.local/state/fnm  fnm   25.5   ⊘ /opt/homebrew/bin brew  24.9     │
│ rg          /opt/homebrew/bin   brew  14.1.1 ⊘ ~/.cargo/bin    cargo   14.1.0   │
│ ruby        /opt/homebrew/bin   brew  3.4    ⊘ /usr/bin        system  2.6      │
╰──────────────────────────────────────────────────────────────────────────────────╯
 /opt/homebrew/bin › ~/.cargo/bin › ~/.local/bin › /usr/local/bin › /usr/bin
```

The footer is `$PATH` itself, in order, as clickable segments — click one to
filter the table to what it provides. The winner column shows *why* it wins: it
is earlier in that list.

Clicking a loser opens detail on the loser, which is usually the moment you
discover you have been running a year-old `rg` from `cargo install` without
noticing.

## What it will not do

An earlier draft of this had `space` to mark rows, drag-marquee selection in the
map, and a bulk uninstall composing one `brew uninstall a b c`. That is now cut,
permanently rather than deferred.

**yoghurt never writes to your machine.** It does not install, uninstall,
upgrade, prune, or touch your shell configuration. The only things it writes are
its own cache and its own config.

This is a stronger promise than any feature it gives up, and it is worth more to
a tool whose whole job is to be trusted with a complete view of your machine.
The detail pane shows the uninstall command as copyable text; running it is your
decision, in your shell, where you can see it.

Two other things are parked for a reason worth writing down:

**Last-used, and the `·` glyph.** "What have I not run in a year" is the most
useful question this could answer, and it may not be answerable. Access times
are the obvious source and they are unreliable — `relatime` and `noatime` are
common, and on some filesystems the field means nothing. A glyph backed by a
number that is silently wrong is worse than no glyph, so it does not ship until
something establishes where a trustworthy signal comes from.

**Linux.** v1.0 says macOS and means it. Homebrew, `/Applications`, `codesign`
and `Info.plist` are the machine this is designed against. The graph and the
`PATH` walk are portable; five more adapters and a different idea of what
"installed" means are not, yet.

## Scanning

The interface never waits on a subprocess.

1. **First frame comes from cache** — `~/.cache/yoghurt/scan.json`. It draws
   immediately and says `scanned 2m ago`, so you always know what you are
   looking at.
2. **Each source refreshes independently** in the background. Rows fill in as
   they land; a source still scanning shows `…` for its count and a shimmer on
   its group header. One slow manager never holds up the other seven.
3. **One subprocess per source, not per package.** `brew info --json=v2
   --installed` returns names, versions, sizes, dependency graph and the
   installed-on-request flag for all 179 packages at once.
4. **Provenance for orphan binaries comes from the binaries.** `go version -m`
   reads the module path and version straight out of a Go binary; Mach-O and
   ELF metadata, `codesign -dv` and `Info.plist` cover most of the rest. An
   orphan that can be identified stops being an orphan.
5. **Network checks are opt-in.** Nothing queries a registry on startup. `r`
   refreshes the outdated set, or a config setting does it on a timer.
6. **The filesystem is watched.** `notify` on the Cellar, Caskroom, `.cargo/bin`
   and the PATH directories invalidates the cache, so a `brew install` in
   another pane appears here without being asked.

## Read-only, permanently

yoghurt is for looking. The tool that surveys your machine is not the tool that
should surprise it, and the way to guarantee that is not a confirmation dialog —
it is not having the code.

## Theme

Roles, not colours. `auto` maps every role onto the terminal's own ANSI palette
and is the default, because yoghurt should look like the terminal it runs in
rather than like somebody else's screenshot. `mono` drops colour entirely and
loses nothing, because every state already has a glyph. Ghostty theme files are
read directly, so a theme you already picked does not need transcribing.

Each source gets one hue, used consistently across all three views: the colour
that means "homebrew" in the list means "homebrew" in the treemap.

## Keys

Every one of these has a mouse equivalent. The mouse is not an afterthought
bolted onto a keyboard interface; both are complete.

```
 ↑↓ jk    move              tab    view            g      group axis
 ↵        inspect           s      sort            m      map metric
 /        filter            !      problems only   a      show all
 r        rescan            e      reveal          ?      help
 esc      back / clear      q      quit
```

## Build order

The backlog is the plan: `cairn roadmap`, or `ROADMAP.md`. Five milestones.

1. **v0.1 — See it.** `PATH` ground truth, the Homebrew adapter, the Inventory
   and the why-chain. Homebrew is 60% of the rows and 90% of the disk, and one
   JSON call gives leaf-against-dependency, which is the insight the whole tool
   rests on. Everything Homebrew does not claim already shows as an orphan.
2. **v0.2 — All of it.** The remaining seven adapters, and a cache so it opens
   instantly.
3. **v0.3 — Contention.** The Path view. Gated on a spike, because it is the
   most speculative part of this document.
4. **v0.4 — The chart.** The Map. Also gated on a spike, because a treemap that
   is illegible at eighty columns is a bar chart with extra steps.
5. **v1.0 — Stand behind it.** No new surface. Packaging, documentation, and the
   promises above written down where somebody can hold us to them.

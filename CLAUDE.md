# Working in this repository

yoghurt surveys every package manager on a machine and presents one inventory.
Rust, terminal interface, read-mostly.

## Never

- **No assistant or AI attribution.** Not in commits, trailers, pull request
  bodies, code comments, docs, changelogs, or release notes. No co-author
  trailers naming a model, no "generated with" footers, no robot emoji. The
  `commit-msg` hook rejects them; do not work around it. This repository is
  published under its author's name.
- **Never commit to `main`.** It advances only through a merged pull request.
  The `pre-push` hook and branch protection both refuse a direct push.
- **Never `--no-verify`, `continue-on-error`, or `|| true`** to get past a red
  check. Fix the cause.

## The seam

All automation talks to this project through one interface. Use these verbs;
never call `cargo` directly in CI, a hook, or a script.

```
scripts/task fmt        format in place
scripts/task fmt:check  verify formatting
scripts/task lint       clippy, warnings denied
scripts/task test       full test suite
scripts/task build      release build
scripts/task install    put it on your PATH from this tree
scripts/task check      all of the above
```

If a stack command needs to change, change it in `scripts/task` only. CI runs
`scripts/task check` and nothing else, so the two cannot drift.

## The workflow

One unit of work is one worktree, one branch, one pull request. Parallel agents
never share a checkout.

```sh
scripts/agent start <type>/<slug>   # type ∈ feat fix chore docs perf refactor test
cd ../.worktrees/yoghurt/<type>/<slug>
# ... work ...
scripts/agent check
scripts/agent commit "<conventional commit>"
scripts/agent pr
# after the pull request is squash-merged:
scripts/agent done
```

`scripts/agent doctor` diagnoses the environment and reports every problem, not
just the first. Run it when something is not behaving.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
`type(scope)!: subject`. Imperative, 72 characters or fewer, no trailing period.
The body says *why*; the diff already says what. `Refs: #<issue>` for a tracker
item. The hook enforces all of it.

## Code

- Make it work, make it right, make it small. In that order.
- Comments say why. The code already says what.
- No dead scaffolding: no TODO stubs, no commented-out code, no abstraction for
  a second caller that does not exist.
- A dependency earns its place or does not get added. Prefer `std`.
- Handle an error where you can act on it; otherwise propagate. Never swallow
  one.
- A bug fix arrives with the test that would have caught it.

## This project in particular

- **Read-only by default.** Anything that uninstalls, prunes, or deletes is
  behind an explicit confirmation, because it is not undoable.
- **Never block the interface on a subprocess.** `brew info` costs seconds and
  sometimes a network round trip. Scanning is asynchronous and fills in as it
  lands; the first frame draws from cache.
- **A source that is absent is absent.** Do not render a package manager the
  machine does not have as a row with a count of zero.
- **Colour is never the only signal.** Every state also has a glyph, so the
  interface still reads under `NO_COLOR`, in a monochrome theme, or to someone
  who cannot tell the green from the red.
- Adapters are per-package-manager and share one interface. Add a source by
  adding an adapter, not by special-casing the interface.

Language rules live in `.claude/rules/` and load when you touch those files.
Formatting is `rustfmt`'s job — do not discuss it in review.

<!-- cairn:begin -->
## Roadmap and issues

This project tracks its roadmap and issues with `cairn`. Every item is a Markdown file under `cairn/items`, described by the schema in `cairn.toml`.

**Do not create ad-hoc TODO, PLAN or NOTES files.** Create a cairn item instead, so the work appears on the board and in the generated roadmap.

### The loop

1. `cairn next` — what is ready to start. It excludes anything blocked by unfinished dependencies and puts work already in progress first.
2. `cairn claim <ID>` — take it before you start, so no one duplicates the work. `cairn claim --next` picks and claims the top-ranked unclaimed item in one step, and prints its body so you can begin immediately.
3. Do the work. Record what you learn: `cairn set <ID> <field>=<value>` for fields, `cairn note <ID> "<TEXT>"` for anything that needs a sentence — why you chose something, what you tried, what to watch for.
4. `cairn close <ID>` when it is done, or `cairn release <ID>` to hand it back.
5. `cairn check` before you report finished. It must pass.

### Commands

```sh
cairn next --json                 # ready work, ranked
cairn claim --next                # take the next ready item
cairn search <TEXT> --json        # titles, bodies and labels
cairn list --json                 # all open items
cairn list --filter 'blocked=false,priority=p0'
cairn show <ID> --json            # one item, including its body
cairn new "<TITLE>" --type <TYPE> --milestone <MILESTONE>
cairn set <ID> status=<STATUS>    # also labels+=x, or any field below
cairn note <ID> "<TEXT>"          # append reasoning; never replaces
cairn close <ID>
cairn check                       # validate; run before finishing
cairn render                      # regenerate ROADMAP.md
```

### Schema

- **Types**: `feature`, `bug`, `chore`, `spike`, `docs`, `milestone`
- **Statuses**: `backlog` (open), `planned` (open), `doing` (active), `blocked` (active), `done` (done), `dropped` (dropped)
- **`due`**: date, YYYY-MM-DD — when a milestone is meant to land
- **`part_of`**: names any items, by id, several allowed — a larger piece of work this belongs to
- **`priority`**: one of p0, p1, p2, p3 — p0 means this milestone does not ship without it
- **`effort`**: one of s, m, l, xl — Rough size, not an estimate
- **`area`**: one of source, graph, scan, chrome, list, path, map, detail, mouse, filter, theme, cli, runtime, config, testing, docs, packaging — Subsystem this touches
- **Milestones**: `v0.1` (due 2026-10-20), `v0.2` (due 2026-12-01), `v0.3` (due 2027-01-12), `v0.4` (due 2027-02-16), `v1.0` (due 2027-03-30), `later`
- **Saved views** (`cairn list --view NAME`): `now`, `next`, `triage`

### Rules

1. Before starting work, find or create the item and set it to an active status.
2. Use the fields above rather than inventing new ones; add new fields to `cairn.toml` first.
3. Never hand-edit the generated roadmap file — change items and run `cairn render`.
4. `cairn check` must pass before the work is considered done.

<!-- cairn:end -->

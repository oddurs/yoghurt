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

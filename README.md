# yoghurt

[![CI](https://github.com/oddurs/yoghurt/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/yoghurt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

See what is installed on this machine, and where it came from.

Homebrew knows about Homebrew. `npm` knows about `npm`. Nothing knows about all
of it at once, so the honest answer to "what is on this laptop" is a shrug and
five commands whose output does not line up. yoghurt is one inventory across
every package manager you actually use — a chart of the machine rather than a
list per tool.

## What it does today

One command, one table. Detection is filesystem-only, so it returns instantly
and never touches the network:

```console
$ yoghurt
  SOURCE          ITEMS  WHERE
  homebrew          169  /opt/homebrew/Cellar
  homebrew casks     10  /opt/homebrew/Caskroom
  cargo              22  ~/.cargo/bin
  rustup              7  ~/.rustup/toolchains
  go                  1  ~/go/bin
  npm global          3  /opt/homebrew/lib/node_modules
  applications       44  /Applications
  applications         1  ~/Applications
  total             257
```

It finds Homebrew formulae and casks, `cargo install` binaries, rustup
toolchains, `go install` binaries, global npm packages, pipx and uv tools, and
macOS application bundles. A source that is not installed is left out rather
than shown as zero.

That is the whole of it so far. The mouse-first terminal interface this is
groundwork for — the inventory, the disk treemap, and the `PATH` resolver — is
designed in [docs/interface.md](docs/interface.md) and planned in
[ROADMAP.md](ROADMAP.md), but not yet built.

yoghurt supports **macOS**. It compiles on Linux and its tests pass there, but
three of its sources — applications, the App Store, and code signatures — have
nothing to read, so that is a portability check rather than support.

yoghurt is read-mostly. Reading is the default; anything that
changes the machine shows you the exact command first and asks you to type a
confirmation. It never touches your shell configuration, and nothing happens
silently.

## Install

From source, with a Rust toolchain installed:

```sh
cargo install --git https://github.com/oddurs/yoghurt
```

Or clone and build:

```sh
git clone https://github.com/oddurs/yoghurt
cd yoghurt
cargo build --release   # binary at target/release/yoghurt
```

## Optional: what things are for

yoghurt can ask a model to sort packages into categories, which is the one thing
it cannot work out by looking. It is **off** unless you switch it on, because
the rest of the tool never touches the network.

`~/.config/yoghurt/config.toml`:

```toml
[taxonomy]
enabled = true
# The key is never kept here. Point at a file that already holds one.
api_key_file = "~/.config/namesync/env"
api_key_env = "OPENROUTER_API_KEY"
model = "anthropic/claude-haiku-4.5"
```

Only names and descriptions are sent — never paths, never versions, never the
shape of your home directory. Answers are cached in
`~/.cache/yoghurt/taxonomy.json`, so it asks once per package ever and two runs
of the same machine group identically. Labels appear with a `~` in front of
them, because they are a guess and everything else yoghurt shows is an
observation.

## Development

One command sets everything up:

```sh
./scripts/setup
```

That wires the tracked git hooks (`core.hooksPath` → `.githooks`) and runs the
environment check. From then on the binary on your `PATH` keeps itself current:
merging something that changes the source reinstalls it, and `yoghurt --version`
reports the commit it was built from so you can always tell. From then on, two scripts carry the whole workflow.

`scripts/task` is the seam every piece of automation talks to — CI and the git
hooks know only these verbs, so they cannot drift from what you run:

| Command | Does |
| --- | --- |
| `scripts/task fmt` | Format in place |
| `scripts/task fmt:check` | Verify formatting |
| `scripts/task lint` | Clippy, warnings denied |
| `scripts/task test` | Full test suite |
| `scripts/task build` | Release build |
| `scripts/task install` | Put it on your PATH from this tree |
| `scripts/task check` | All of the above |

`scripts/agent` is the workflow — one unit of work, one worktree, one branch,
one pull request:

| Command | Does |
| --- | --- |
| `scripts/agent doctor` | Check tooling, auth, hooks, working tree |
| `scripts/agent start <type>/<slug>` | Branch and worktree from `main` |
| `scripts/agent check` | `scripts/task check` |
| `scripts/agent commit <msg>` | Commit, with the message validated |
| `scripts/agent pr [--draft]` | Check, push, open the pull request |
| `scripts/agent sync` | Rebase onto `origin/main` |
| `scripts/agent done` | Confirm merged, then clean up |
| `scripts/agent list` | Worktrees, branches, pull request state |

A full pass looks like this:

```sh
scripts/agent start feat/treemap
cd ../.worktrees/yoghurt/feat/treemap
# ... work ...
scripts/agent commit "feat: draw the disk treemap"
scripts/agent pr
# ... review, squash-merge ...
scripts/agent done
```

`main` advances only through a merged pull request. The `pre-push` hook refuses
a direct push, and branch protection refuses it again on the server.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the details.

## License

[MIT](LICENSE) © Oddur Sigurdsson

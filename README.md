# yoghurt

[![CI](https://github.com/oddurs/yoghurt/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/yoghurt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

See what is installed on this machine, and where it came from.

Homebrew knows about Homebrew. `npm` knows about `npm`. Nothing knows about all
of it at once, so the honest answer to "what is on this laptop" is a shrug and
five commands whose output does not line up.

```
 yoghurt  weezer   313 packages · 9 sources · 43G                          scanned just now 
 171 wanted   132 pulled in   44 outdated   0 unexplained   3 broken   10 system            
─ by role · size↓ ──────────────────────────────────────────────────────────────────────────
 ▾ wanted                                                                         171  35G  
  ● iMovie                                                             -    wanted    4.0G  
  ● Docker                                                        4.61.0    wanted    2.4G  
  ● stable-aarch64-apple-darwin                                        -    wanted    2.2G  
  ● Google Chrome                                           153.0.8010.…    wanted    2.1G  
  ● GarageBand                                                         -    wanted    1.5G  
  ● Visual Studio Code                                           1.137.0    wanted    1.4G  
  ● 1.98-aarch64-apple-darwin                                          -    wanted    1.2G  
  ● 1.96.1-aarch64-apple-darwin                                        -    wanted    1.1G  
  ● UTM                                                            4.7.5    wanted    1.1G  
 ↑↓ move  space fold  g group  s sort  / find  ! facet  ↵ detail  r rescan  q quit          
```

Three numbers on that screen are the reason this exists. **163 things you asked
for. 133 that arrived underneath something else. 4 that nothing you installed
needs at all** — the residue of uninstalls that did not finish, which no package
manager can see on its own because each of them only knows its own half.

## Why is this here?

Press `↵` on anything and it answers the question a flat list cannot:

```
 yoghurt  weezer   313 packages · 9 sources · 43G                                      scanned just now 
 171 wanted   132 pulled in   44 outdated   0 unexplained   3 broken   10 system                        
─ /glib · by source ────────────────────────────────────────────────────────────────────────────────────
 ▾ homebrew                                            2  42M   glib                                    
  ◐ glib                                               2.88.3  ◐ 2.88.3 · homebrew · 38M                
  ◐ libtool                                             2.6.2  Core application library for C           
                                                                                                        
                                                                WHY ──────────────────────────────────  
                                                                 glib                                   
                                                                   └ epubcheck  ← you installed this    
                                                                                                        
                                                                FACTS ────────────────────────────────  
 ↑↓ move  space fold  g group  s sort  / find  ! facet  ↵ detail  r rescan  q quit                      
```

`glib` is 151 MB you never asked for. It is here because `epubcheck` is, and
that is the thing you actually installed. No package manager answers that in one
step.

## What it reads

Homebrew formulae and casks, `cargo install` binaries, rustup toolchains, and
every application bundle — including which ones came from the Mac App Store and
which vendor signed the rest.

Anything left over is reported as unclaimed, honestly, rather than hidden. On
the machine above that is two binaries `cargo` never recorded installing.

## Install

```sh
brew install oddurs/tap/yoghurt   # once the tap exists
cargo install --git https://github.com/oddurs/yoghurt
```

yoghurt supports **macOS**. It compiles on Linux and its tests pass there, but
three of its sources — applications, the App Store, and code signatures — have
nothing to read, so that is a portability check rather than support.

## Using it

| | |
| --- | --- |
| `↑↓` `jk` | move |
| `↵` | detail, and why it is here |
| `space` | fold a group |
| `g` | group: source, role, category, purpose, size, age, health |
| `s` `S` | sort column, reverse |
| `/` | find by name, source, command, or description |
| `!` | filter by one of the counts in the strip |
| `r` `R` | rescan; `R` also asks the network what is newer |
| `esc` | undo one narrowing, then leave |

**The mouse does all of it.** Click a count to filter, a heading to fold, the
rule to regroup, a key in the footer to press it. Hovering marks a row without
selecting it.

Piped, it prints a table instead, so `yoghurt | awk` works:

```console
$ yoghurt | grep pulled-in | wc -l
133
```

`yoghurt --screenshot 92x14 --group role` renders one frame and exits, which is
how the screens above were made.

## Read-mostly

Reading is the default and almost all of what this does. Anything that changes
the machine shows the exact command first and asks you to type a confirmation —
never a keystroke, and never in bulk without the whole list visible. It never
touches your shell configuration, and it never contacts the network unless you
ask it to.

## Optional: what things are for

yoghurt can ask a model to sort packages into categories, which is the one thing
it cannot work out by looking. It is **off** unless you switch it on.

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
shape of your home directory. Answers are cached, so it asks once per package
ever and two runs of the same machine group identically. Labels appear with a
`~`, because they are a guess and everything else here is an observation.

## Development

```sh
./scripts/setup
```

That wires the tracked git hooks and runs the environment check. From then on
the binary on your `PATH` keeps itself current — **committing** installs what
you just wrote, and **merging** installs what you just merged, so what you run
is always what you are looking at. Neither costs anything when the commit did
not touch the source. `yoghurt --version` reports the commit it was built
from.

`scripts/task` is the seam every piece of automation talks to — CI and the git
hooks know only these verbs:

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

```sh
scripts/agent start feat/npm-source
cd ../.worktrees/yoghurt/feat/npm-source
scripts/agent commit "feat: read global npm packages"
scripts/agent pr
# after the pull request is squash-merged:
scripts/agent done
```

`main` advances only through a merged pull request. See
[CONTRIBUTING.md](CONTRIBUTING.md), the design in
[docs/interface.md](docs/interface.md), and the plan in [ROADMAP.md](ROADMAP.md).

## License

[MIT](LICENSE) © Oddur Sigurdsson

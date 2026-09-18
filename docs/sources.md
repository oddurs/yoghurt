# Adding a source

A source is one file. It knows nothing about the interface, nothing about the
other sources, and nothing about how anything is scored or shown — so adding a
package manager touches one file and no others.

That claim is checked rather than asserted: `tests/discipline.rs` fails if an
adapter cannot be pointed at a fixture, has no tests, reads the machine it is
running on, or shells out from a test.

## The contract

```rust
pub trait Source {
    fn name(&self) -> &'static str;
    fn scan(&self) -> Result<Vec<Fact>, ScanError>;
    fn updates(&self) -> Result<Vec<Fact>, ScanError> { Ok(Vec::new()) }
}
```

`scan` says what is installed. `updates` says what is newer and is separate
because for most sources it is a network round trip — yoghurt does not touch the
network unless asked, so `r` rescans locally and `R` also asks.

## What a source may say

Facts are additive and order-independent. Two sources claiming one artifact is a
thing that happens on a real machine, so it is representable rather than an
error; the graph reconciles them.

| Fact | Means |
| --- | --- |
| `Package` | this exists, at this version |
| `Wanted` | somebody asked for it — its **absence** is what makes something "pulled in" |
| `System` | the operating system shipped it; nobody chose it and nobody can remove it |
| `Owns` | this package put this path on disk |
| `DependsOn` | this package needs another |
| `Provides` | this artifact runs under this command name |
| `Size` | bytes |
| `InstalledAt` | when |
| `Describes` | what it is for, in a sentence a human wrote |
| `Outdated` / `UpToDate` | newer exists / somebody looked and this is newest |

Nothing here is a conclusion. "Orphan", "shadowed", "pulled in" and "broken" are
questions the graph answers by looking at what is and is not connected — so a
package manager this program has never heard of gets all four for nothing.

## The rules

**Never claim what you did not install.** The Homebrew adapter walks the Cellar
rather than trusting `brew info --json`, because that JSON silently omits
packages from untrusted taps — and an adapter built on it alone would have lost
three and then shown them as orphans.

**Never invent a package.** A fact about a name that is not installed does not
update a package, it creates one. The rustup adapter filters its update report
down to toolchains that exist on disk for exactly this reason.

**Unknown beats wrong.** A source with no uninstall command offers none. A
package whose version cannot be read has no version. "Not checked" and "current"
are different answers and must never share a glyph.

**Absent is not an error.** A package manager that is not installed returns no
facts. Only a source that *exists and could not be read* returns `ScanError`,
and even then the other sources still finish — one failure never takes the scan
down.

**No subprocess per item.** One `brew info` for 224 packages, one
`go version -m` for every binary at once. `codesign` runs once per bundle and
never per file: 45 bundles is 0.8 seconds where 3000 artifacts would be a
minute.

## Writing one

Take `src/source/node.rs` as the shape. It is about 150 lines including its
tests, needs no subprocess, and handles the three things that are easy to get
wrong in that ecosystem — scoped packages living a directory deeper, a `bin`
field that is either a map or a bare string, and `.bin` being symlinks rather
than a package.

```rust
pub struct Thing { root: PathBuf }

impl Thing {
    pub const NAME: &'static str = "thing";

    /// This machine.
    pub fn from_environment() -> Option<Self> { /* … */ }

    /// Somewhere else. This is what the tests use, and it is why they can.
    pub fn new(root: PathBuf) -> Self { Self { root } }
}
```

Then register it in `survey::sources()`, and nothing else changes.

## Testing one

Build a fixture tree in a scratch directory that removes itself, point the
adapter at it, assemble a graph from the facts, and assert against the graph
rather than against the fact list — the graph is what the rest of the program
sees.

Anything that would ask the machine a question — `codesign`, `rustup check`,
`go version -m` — is a function the constructor takes, so a test supplies the
answer. The suite runs in under two seconds and passes on a machine with no
package managers installed at all.

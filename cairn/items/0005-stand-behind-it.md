---
id: 5
key: v1.0
title: Stand behind it
type: milestone
status: backlog
created: 2026-09-13
updated: 2026-09-13
priority: p2
due: 2027-03-30
---

## Ships

No new surface. The version somebody who is not Oddur can install in one
command, run on their own machine, and rely on.

## The promise

yoghurt shows you what is installed on your Mac and where it came from. It
never changes anything without showing you the command and asking you to type a
confirmation, and never talks to the network unless you ask it to.

## Done when

- [ ] `brew install oddurs/tap/yoghurt` works, and so does `cargo install yoghurt`
- [ ] A tagged push builds and publishes binaries for Apple silicon and Intel
- [ ] A machine with nothing installed shows an empty state, not a crash
- [ ] A machine with five thousand packages stays responsive
- [ ] The terminal is never left broken — not on panic, not on SIGTERM, not on
      a resize during a scan
- [ ] The README lets a stranger install it and understand the first screen
- [ ] Every key and every click is documented
- [ ] What yoghurt will never do is written down

## Explicitly not in this milestone

Nothing new. If something here is exciting, it is in the wrong milestone.

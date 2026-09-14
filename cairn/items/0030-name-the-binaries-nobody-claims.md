---
id: 30
title: Name the binaries nobody claims
type: feature
status: backlog
milestone: v0.2
depends_on:
- 24
- 29
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: source
---

## Problem

After every adapter runs, what is left is the orphan set. Showing it as a list
of filenames is honest but unhelpful.

## Proposal

Apply the identification methods the spike ranked, best hit rate first, and stop
at the first that answers. An orphan that can be identified stops being an
orphan and becomes a package with an unusual source. One that cannot says
plainly what is known: path, size, when it appeared, whether it is signed.

## Acceptance criteria

- [ ] Identification is tried in the ranked order and stops at the first answer
- [ ] An identified orphan moves out of the orphan facet
- [ ] An unidentified orphan shows path, size, first-seen date and signing status
- [ ] Identification never costs more than one subprocess per binary
- [ ] The whole orphan pass over 600 binaries stays under two seconds

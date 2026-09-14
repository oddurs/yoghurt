---
id: 68
title: Correct the claims the Homebrew spike invalidated
type: chore
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p0
effort: s
area: docs
---

## Problem

Reviewing the merged spike found that its answer is wrong in several places, and
that it left claims standing elsewhere which it had already disproved. A backlog
that lies is worse than no backlog, and three of these would have been
discovered mid-implementation of 0011.

- The spike says the numbers are "corrected in 0023". They are not: 0023 is the
  README rewrite and none of its criteria mention counts. The stale figures live
  in `docs/interface.md`, which no item touches.
- It closed without answering one of its own questions — whether the JSON names
  the binaries a formula provides. It does not.
- It hardcodes `installed[0]`, which is wrong for 44 of 210 formulae here.
- Its explanation of the 210/213 gap is a guess, and the guess is wrong.
- Its size path sums every keg of a formula rather than the one installed.
- 0011's proposal still says casks are formulae with a different label, which
  the spike disproved, and its acceptance criteria can be satisfied while
  emitting no sizes and dropping every cask.

## Proposal

Rewrite the spike's answer against what was actually measured, rewrite 0011's
proposal and acceptance criteria to match, and fix the numbers in
`docs/interface.md`.

## Acceptance criteria

- [ ] The spike answer states what the JSON does not carry: binaries, sizes, and
      packages from untrusted taps
- [ ] Multiple installed kegs per formula are described, with the count measured
- [ ] The 210/213 gap has its real cause recorded
- [ ] 0011's proposal no longer claims casks behave like formulae
- [ ] 0011 has acceptance criteria covering casks, sizes and untrusted taps
- [ ] `docs/interface.md` quotes numbers this machine actually reports

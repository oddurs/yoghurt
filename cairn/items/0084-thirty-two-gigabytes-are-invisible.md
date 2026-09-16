---
id: 84
title: Thirty-two gigabytes are invisible
type: bug
status: done
milestone: v0.3
assignee: Oddur Sigurdsson
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: m
area: source
---

## What happens

The header says the machine holds 12G. Applications hold 23.7G and rustup
toolchains another 8.1G, and both report nothing — 60 rows show no size at all,
and every total that includes them is wrong.

## Why it was left

0010 deliberately did not walk application bundles: `Xcode.app` alone is 14G and
measuring every bundle costs 2.6 seconds. The same reasoning left rustup's eight
toolchains unmeasured at 3.3 seconds.

That was right when the scan was sequential and there was no cache. Neither is
true now: sources run concurrently, so the cost is the slowest rather than the
sum, and the cache means it is paid once rather than on every run.

## Acceptance criteria

- [ ] Application bundles carry a size
- [ ] App Store applications carry a size
- [ ] rustup toolchains carry a size
- [ ] The header total accounts for them
- [ ] The scan stays under five seconds cold, and instant from cache

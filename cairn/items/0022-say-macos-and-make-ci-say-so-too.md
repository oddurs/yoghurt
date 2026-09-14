---
id: 22
title: Say macOS, and make CI say so too
type: chore
status: backlog
milestone: v0.1
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: packaging
---

## Problem

CI runs a `ubuntu-latest` job, and v1.0 promises macOS only. Testing a platform
nobody supports is a job that can only produce misleading failures.

## Proposal

Decide and make the repository agree with itself. Either drop the Linux job, or
keep it explicitly as a compile-only check with the support policy written down.
Do not leave it ambiguous.

## Acceptance criteria

- [ ] `.github/workflows/ci.yml` matches the stated support policy
- [ ] `README.md` says which platforms are supported
- [ ] `Cargo.toml` metadata does not claim platforms that are not tested
- [ ] Linux support exists as an item in `later`, not as an implied promise

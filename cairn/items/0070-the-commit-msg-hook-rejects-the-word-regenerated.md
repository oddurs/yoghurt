---
id: 70
title: The commit-msg hook rejects the word regenerated
type: bug
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-15
priority: p2
effort: s
area: packaging
---

## What happens

A commit body containing the phrase "can be regenerated with" is rejected as
assistant attribution. The attribution pattern includes `generated with`, and
that is a substring of `regenerated with`.

## What should happen

The check should reject the footer phrase and leave ordinary English alone.

## Reproduction

1. Write a commit body containing "The fixture can be regenerated with a flag"
2. `git commit`
3. The hook refuses it, citing assistant attribution

## Fix

Anchor the alternative on a word boundary: `\bgenerated with`. In `regenerated`
the position before `generated` sits between two word characters, so `\b` does
not match there, while `Generated with` at the start of a footer still does.

Add the phrase to the hook's own test cases, both as a message that must pass
and as one that must still be rejected.

## 2026-09-15

Fixing this uncovered a second bug in the same check: the robot emoji test was written as grep '\xf0\x9f\xa4\x96' in single quotes, which searches for that literal text rather than those bytes, so it had never fired once. Both fixed, and the hook is now exercised against four messages.

---
id: 14
title: Render the interface into a buffer a test can read
type: chore
status: done
milestone: v0.1
depends_on:
- 7
created: 2026-09-13
updated: 2026-09-14
priority: p0
effort: m
area: testing
---

## Problem

A terminal interface that can only be tested by looking at it will not be
tested. This is the item that gets left out, and leaving it out is why the
milestones after this one slow down.

## Proposal

A test harness that builds an app state from a fixed fact set, renders a frame
into a ratatui buffer at a given size, and returns it as lines of text. Then an
assertion is a string comparison, and a regression is a diff.

The same harness feeds mouse and key events in, so interaction is testable
without a terminal at all.

## Acceptance criteria

- [ ] `render(app, width, height) -> Vec<String>` with no terminal involved
- [ ] Events can be fed in and the resulting state asserted
- [ ] At least one test renders a full frame at 100x30 and compares it to a fixture
- [ ] Runs in CI on a machine with no Homebrew and no TTY
- [ ] Adding a view later does not require changing the harness

## 2026-09-14

Built together with 0016 in one branch. A harness that renders nothing cannot be tested, and its own criterion asks for a test that renders a full frame — so it needs a view to exist. The chrome is the smallest real one.

---
id: 79
title: Classify packages through OpenRouter, strictly opt in
type: feature
status: backlog
milestone: later
depends_on:
- 78
created: 2026-09-14
updated: 2026-09-14
priority: p3
effort: m
area: source
---

## Problem

yoghurt can say what a thing *is* structurally — tool, library, application —
but not what it is *for*. A crude keyword pass over Homebrew's descriptions
classifies 91 of 210, which is not enough to group by. The gap between "Codec
library for encoding and decoding AV1 video streams" and the label *media* is
semantic, and that is what a model is actually good at.

## Proposal

An optional taxonomy, through OpenRouter, under conditions that keep the rest of
the tool's promises intact.

**Off by default, and never on first run.** The project's own stated promise is
that it never contacts the network unless asked. A prompt that appears the first
time somebody presses the taxonomy axis, explaining what would be sent.

**Cached, so it is deterministic.** One call per package, ever, written to
`~/.cache/yoghurt/taxonomy.json` keyed by name and description. A tool whose
value is telling you the truth about your machine cannot group it differently
between two runs.

**Labelled as inferred.** Everything else yoghurt shows is observed. A model's
label is a guess, and mixing the two silently would corrode the whole
proposition. The fact carries its origin, the axis marks inferred labels, and
the detail pane says where the label came from.

**Only names and descriptions leave the machine.** Never paths, never versions,
never the shape of somebody's home directory. Somebody's installed software says
a lot about them, and that belongs in the opt-in prompt rather than in a
footnote.

## Acceptance criteria

- [ ] Nothing is sent without an explicit opt-in recorded in config
- [ ] The prompt states exactly what would be sent and to whom
- [ ] Results are cached; a second run makes no calls and groups identically
- [ ] Labels are marked as inferred wherever they appear
- [ ] The key comes from config or `OPENROUTER_API_KEY`, never from a flag that
      would land in shell history
- [ ] With the network unavailable, the axis degrades to the structural
      category rather than failing
- [ ] No test makes a network call

# Contributing

Thanks for taking the time. This repository is small and strict on purpose:
the workflow makes the wrong thing hard rather than merely discouraged.

## Once, after cloning

```sh
./scripts/setup
```

This wires the tracked git hooks and checks your toolchain. Without it the
hooks do not run and your first push will bounce off CI instead.

## The loop

One unit of work is one worktree, one branch, and one pull request. Parallel
work never shares a checkout.

```sh
scripts/agent start fix/handle-empty-input
cd ../.worktrees/yoghurt/fix/handle-empty-input
# ... change code, add the test that would have caught the bug ...
scripts/agent check
scripts/agent commit "fix: handle empty input"
scripts/agent pr
```

After the pull request is squash-merged:

```sh
scripts/agent done
```

That removes the worktree, deletes the local branch, and returns you to `main`.

## Branch names

`<type>/<slug>`, where type is one of `feat` `fix` `chore` `docs` `perf`
`refactor` `test`. Slugs are lower-case, hyphen-separated. `scripts/agent start`
refuses anything else.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). The
`commit-msg` hook enforces it:

```
<type>(<optional scope>)<optional !>: <subject>

<body: why the change, not what the diff already says>

Refs: #<issue>
```

- Subject in the imperative, 72 characters or fewer, no trailing period.
- `!` after the type or scope marks a breaking change.
- No assistant or AI attribution — no co-author trailers naming a model, no
  "generated with" footers. The hook rejects them. Work published here goes
  out under its author's name.

## Green before it is a pull request

```sh
scripts/task check   # fmt:check, lint (warnings denied), test, build
```

CI runs exactly this command, so a green local check means a green pipeline.
The `pre-commit` hook runs formatting and lint; `pre-push` runs the full check
and refuses a push to `main`.

Never use `--no-verify`, `continue-on-error`, or `|| true` to get past a red
check. Fix the cause.

## Bug fixes

A bug fix arrives with the test that would have caught it.

## Review

This is currently a solo repository, so branch protection requires **zero**
approvals — the owner would otherwise be deadlocked. Everything else still
applies: a pull request is mandatory, the `required` status check must pass,
the branch must be up to date, and conversations must be resolved. If the
project gains a second maintainer, raise the required approvals to one.

## Reporting security issues

Do not open a public issue. See [SECURITY.md](SECURITY.md).

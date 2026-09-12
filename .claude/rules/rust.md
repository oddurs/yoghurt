---
paths: ["**/*.rs", "Cargo.toml", "rust-toolchain.toml"]
---

# Rust in this repository

- Edition 2024, toolchain pinned in `rust-toolchain.toml`. Do not bump it in a
  change that is about something else.
- `unsafe_code` is forbidden at the crate level. There is no exception worth
  taking for a tool that reads directory entries.
- Clippy runs with `-D warnings` and `pedantic` enabled. Fix the lint rather
  than silencing it; if a lint is genuinely wrong, `#[expect(...)]` with a
  comment saying why, never a bare `#[allow]`.
- Public items carry a doc comment — `missing_docs` is on. A fallible public
  function documents its `# Errors`.
- Prefer `std`. A new dependency needs a sentence in the commit body arguing for
  it.
- Errors: return a type the caller can act on. In the binary, a message on
  stderr and a non-zero `ExitCode`; never `unwrap` outside tests.
- Tests live next to the code in `mod tests`, integration tests in `tests/`.
  Name a test after the behaviour it pins, not the function it calls.
- No test touches the real machine's package managers. Detection takes a path;
  point it at a scratch directory.

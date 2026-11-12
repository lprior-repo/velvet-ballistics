# Bead vb-af5iz: Source-Only Clippy Target Alignment

## Problem
User AGENTS.md states: "Source lint is zero tolerance. Tests must compile and run, but test clippy is not strict." The actual Moon task `lint-src` already uses `--lib --bins --examples`. However, multiple documentation files still specify `--all-targets` on cargo clippy command lines, creating an inconsistency between the user policy, the actual CI implementation, and the documentation.

## Decision
Align all documented cargo clippy commands to use `--lib --bins --examples` instead of `--all-targets`. This is a documentation-only alignment that reflects the existing CI reality and user intent. The change affects 6 allowed documents with 8 matching cargo clippy command lines.

## Scope
Change only the `--all-targets` token on cargo clippy command lines/explicit clippy policy commands to `--lib --bins --examples`. Preserve every other byte on command lines: `--all-features`, `-D warnings`, existing nightly spec, other flags, crate args, row format, and footnotes.

## Exclusions
- `cargo check --all-targets` commands remain unchanged (typecheck coverage is intentionally broad)
- `cargo test --all-targets` commands remain unchanged (test coverage is intentionally broad)
- Proof/verifier commands remain unchanged
- Non-clippy prose, examples, feature whitelist, and nightly pin remain unchanged
- No code, config, or test files are modified

## Rationale
Source-only clippy includes production examples and all features; typecheck and test coverage remains broad via their own dedicated `--all-targets` commands. Canonical master authority is retained; this is an explicit user policy alignment.

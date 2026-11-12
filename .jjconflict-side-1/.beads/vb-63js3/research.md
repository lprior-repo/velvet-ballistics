# Bead vb-63js3: Research and Audit

## Historical Context

This bead routes Flux lane execution through the existing wrapper script
`scripts/flux-check-package.sh`. The wrapper was authored in a parent audit bead
and has not been modified by this bead.

## Parent Audit Findings

The parent audit identified four own hunks across three files. This child bead
only addresses the xtask routing with its test suite. It does not touch the bulk
wrapper behavior or the Moon lane restoration work, which remain in separate
beads.

## Corrections from Root Instructions

Several root-level instructions were found to be incorrect and have been corrected
in the contract:

1. **Line count**: The original `lanes.rs` is exactly 201 physical lines, not 224.
2. **Return type**: `lane_command` returns `Vec<String>`, not `Result<String>`.
   There is no `execute_lane` function in this module.
3. **Test commands**: `moon run :proofs flux` does not exist as a task. The actual
   verification command is `cargo test --manifest-path xtask/Cargo.toml --bin xtask
   lanes::tests --locked`. `cargo test -p xtask --lib` is invalid because xtask is
   outside the workspace and is binary-only.

## Wrapper Command Format

The xtask lane command constructs three argv elements for bash. The wrapper script
internally adds `--message-format human` when invoking `cargo flux`. The effective
command differs from the xtask argv list. This wrapper routing is intentional and
preserves the existing script behavior.

## Path Handling Limitations

- Relative workspace roots produce relative script paths in the command.
- `display().to_string()` is lossy for non-UTF-8 paths.
- No unconditional absolute path guarantee is provided or tested.

## Test Coverage

The four new tests verify:
- Exact command construction for the Flux lane (Vec equality)
- Workspace root paths with spaces
- Workspace root paths with shell metacharacters (`$`, `(`, `)`, `&`, space)
- Package names with shell metacharacters retained as single argv elements

All tests use exact Vec equality assertions. No tests execute the constructed
commands. No unchecked indexing is used.

## Non-Claims

- The wrapper script is not claimed to be well-tested by this bead.
- No claim is made that the Flux check actually passes for any crate.
- No proof closure or verification obligation is closed by this bead.

S-21 cli-matrix-conformance: Audit (do not create) the cli_matrix_conformance proptest per master §33.3. PROBE-DRIVEN, not CREATE-DRIVEN.

# Verification excerpts (read-before-write)

## Master doc §33.3 (lines 1419-1432) — VERBATIM
The 6 sources of truth with exact paths:
| Source | Path | Coverage | Status |
|--------|------|----------|--------|
| `Command` enum | `crates/vb_cli/src/args/types.rs:67-215` | 30/30 | Matches matrix exactly. |
| `VALID_COMMANDS` const | `crates/vb_cli/src/args/types.rs:230` | 30/30 | Matches matrix exactly. |
| `parse_args` dispatch | `crates/vb_cli/src/args/shared.rs:208-254` | 30/30 | Matches matrix exactly. |
| `run_from_env` dispatcher | `crates/vb_cli/src/dispatcher.rs:49-159` | 30/30 | Matches matrix exactly. |
| `HELP` string | `crates/vb_cli/src/constants.rs:8-53` | 30/30 | Matches matrix exactly. |
| `agent_context::commands()` JSON | `crates/vb_cli/src/agent_context/mod.rs:103-260` | **22/30** | **GAP — 7 entries missing (see §33.4).** |

The matrix-conformance proptest (`crates/workspace_tests/tests/cli_matrix_conformance.rs`) asserts that all six sources stay in lockstep with §33.1.

## CRITICAL FINDING: the proptest does NOT exist
A repo-wide search (`find . -name "cli_matrix*"` and `find . -name "*matrix*"` filtered) returns ZERO files matching `cli_matrix_conformance.rs`. The master doc claims it exists, but it does NOT — this is a master-doc drift. The 6 sources themselves DO exist:
- `crates/vb_cli/src/args/types.rs:70-217` (Command enum, 30 variants)
- `crates/vb_cli/src/args/types.rs:232` (VALID_COMMANDS)
- `crates/vb_cli/src/args/shared.rs` (parse_args dispatch)
- `crates/vb_cli/src/dispatcher.rs` (run_from_env — verified at `dispatcher.rs:49-159` per master)
- `crates/vb_cli/src/constants.rs` (HELP)
- `crates/vb_cli/src/agent_context/mod.rs:103-260` (commands)

# Round-3 corrections applied (from black-hat review)

The round-2 bead had:
1. Wrong file paths (`vb_cli/src/args/constants.rs` — should be `args/types.rs:230`; `args/parse.rs` — should be `args/shared.rs`; `lib.rs::Command` — should be `args/types.rs:67-215`).
2. Priority drift: CUE body said P3, bd list said P0. Black-hat recommended reverting to P1 to avoid priority inversion.

The round-3 corrections:
- **Priority: P1** (not P0). S-21 is a S-class maintenance bead, not a P0 blocker.
- **Scope: PROBE-DRIVEN, not CREATE-DRIVEN.** Master doc says the proptest exists; it doesn't. This bead first AUDITS whether the proptest exists, and if not, it CREATES it.
- **File paths**: Use master §33.3's exact paths.
- **Remove all priority-inversion deps** on P0-2r (vb-riz9e), P0-3r (vb-ujho9), P1-13 (vb-qwsyi). These don't actually need S-21 to land first.

# Scope (verified, no fabrication)

Phase 1 — Audit (read-only):
- Search for `crates/workspace_tests/tests/cli_matrix_conformance.rs`. If absent, the master doc is wrong.
- Read each of the 6 source files and assert the 6 sources stay in lockstep with §33.1.
- The agent_context gap (22/30) is documented in master §33.4; not a fix scope of this bead.

Phase 2 — Create the proptest (if missing):
- Create `crates/workspace_tests/tests/cli_matrix_conformance.rs` with the 6 cross-reference assertions.
- Wire it in `crates/workspace_tests/Cargo.toml` (add `[[test]] name = "cli_matrix_conformance"`).

# Acceptance test

```rust
// In crates/workspace_tests/tests/cli_matrix_conformance.rs
#[test]
fn command_enum_matches_valid_commands() {
    use vb_cli::args::types::{Command, VALID_COMMANDS};
    let tokens: Vec<&str> = VALID_COMMANDS.split(", ").collect();
    assert_eq!(tokens.len(), 30);
    // ... assert each token corresponds to a Command variant
}
```

# Anti-hallucination guards

- DO NOT use the round-2's wrong file paths (`args/constants.rs`, `args/parse.rs`, `lib.rs::Command`).
- DO NOT claim the proptest exists when it doesn't — first audit, then create if missing.
- DO NOT set priority to P0 — this is a S-class maintenance bead. Set to P1.

# Kani harness (skipped — this is a conformance test, not a runtime contract)

# Dependency

This bead has NO dependencies. (Round-2 had vb-riz9e (P0-2r), vb-ujho9 (P0-3r), vb-qwsyi (P1-13) blocked on this — those were priority inversions. Already removed in round 3 dep cleanup.)

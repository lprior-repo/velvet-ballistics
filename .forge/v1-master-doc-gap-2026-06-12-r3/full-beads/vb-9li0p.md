# S-21 cli-matrix-conformance

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> master doc §33.3 (lines 1419-1432), 6 source files enumerated in master §33.3.

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-3 corrections applied):
- Priority: **P1** (NOT P0; this is a S-class maintenance bead).
- Scope: PROBE-DRIVEN, not CREATE-DRIVEN. The master doc claims the proptest exists; the audit first determines if it does, then creates it if missing.
- File paths: Use master §33.3's exact paths (NOT the round-2's wrong paths).
- Dependencies: REMOVED. The round-2 had vb-riz9e, vb-ujho9, vb-qwsyi blocked on this — those were priority inversions; removed.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL audit (read-only) whether the `cli_matrix_conformance` proptest exists at `crates/workspace_tests/tests/cli_matrix_conformance.rs`.
- THE SYSTEM SHALL, if the proptest is missing, create it with the 6 cross-reference assertions from master §33.3.
- THE SYSTEM SHALL set priority to P1 (NOT P0; this is a S-class maintenance bead).
- THE SYSTEM SHALL use the exact file paths from master §33.3 (NOT the round-2's wrong paths).

### Event-Driven
- WHEN the audit finds the proptest EXISTS, THE SYSTEM SHALL close the bead with a `bd remember` note documenting the audit.
- WHEN the audit finds the proptest is MISSING, THE SYSTEM SHALL create it at `crates/workspace_tests/tests/cli_matrix_conformance.rs`.
- WHEN the proptest is created, THE SYSTEM SHALL wire it in `crates/workspace_tests/Cargo.toml`.

### Unwanted
- THE SYSTEM SHALL NOT use the round-2's wrong file paths (`args/constants.rs`, `args/parse.rs`, `lib.rs::Command`).
- THE SYSTEM SHALL NOT claim the proptest exists when it doesn't — first audit, then create if missing.
- THE SYSTEM SHALL NOT set priority to P0 — this is a S-class maintenance bead. Set to P1.
- THE SYSTEM SHALL NOT add priority-inversion deps on P0-2r (vb-riz9e), P0-3r (vb-ujho9), P1-13 (vb-qwsyi).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs: None (this is an audit + optional creation bead).
- system_state:
  - Master doc §33.3 (lines 1419-1432) enumerates the 6 sources of truth.
  - The 6 sources themselves DO exist:
    1. `Command` enum at `crates/vb_cli/src/args/types.rs:67-215` (30 variants)
    2. `VALID_COMMANDS` const at `crates/vb_cli/src/args/types.rs:230`
    3. `parse_args` dispatch at `crates/vb_cli/src/args/shared.rs:208-254`
    4. `run_from_env` dispatcher at `crates/vb_cli/src/dispatcher.rs:49-159`
    5. `HELP` string at `crates/vb_cli/src/constants.rs:8-53`
    6. `agent_context::commands()` JSON at `crates/vb_cli/src/agent_context/mod.rs:103-260` (22/30; 7 entries missing per §33.4)

### Postconditions
- state_changes:
  - If the proptest is missing: create `crates/workspace_tests/tests/cli_matrix_conformance.rs` and wire it in `Cargo.toml`.
  - The audit result is recorded in a `bd remember` note.
- return_guarantees:
  - field: `audit result`
    guarantee: One of "EXISTS" (close bead) or "MISSING — created" (create the proptest).
- side_effects: None (read-only audit, plus the proptest creation if needed).

### Invariants
- The 6 sources of truth stay in lockstep with master §33.1 (the CLI matrix).
- The proptest asserts that the 6 sources agree on the 30 commands (or 22/30 for `agent_context`, per the documented gap in §33.4).
- This bead has NO dependencies (round-2 had priority inversions; removed).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: master doc §33.3 (lines 1419-1432)
  what_to_extract: The 6 sources of truth with their exact paths. Confirmed.
  document_in: research_notes.md
- path: `crates/vb_cli/src/args/types.rs:67-215`
  what_to_extract: The `Command` enum. Confirm 30 variants.
  document_in: research_notes.md
- path: `crates/vb_cli/src/args/types.rs:230`
  what_to_extract: The `VALID_COMMANDS` const. Confirm it has 30 entries.
  document_in: research_notes.md
- path: `crates/vb_cli/src/args/shared.rs:208-254`
  what_to_extract: The `parse_args` dispatch.
  document_in: research_notes.md
- path: `crates/vb_cli/src/dispatcher.rs:49-159`
  what_to_extract: The `run_from_env` dispatcher.
  document_in: research_notes.md
- path: `crates/vb_cli/src/constants.rs:8-53`
  what_to_extract: The `HELP` string. Confirm it lists 30 commands.
  document_in: research_notes.md
- path: `crates/vb_cli/src/agent_context/mod.rs:103-260`
  what_to_extract: The `commands()` function. Confirm 22/30 entries (7 missing per §33.4).
  document_in: research_notes.md

Patterns to find:
- pattern: `cli_matrix_conformance`
  purpose: Locate any existing proptest with this name.
  expected_locations: NONE (the proptest does not exist; this is the audit conclusion).
- pattern: `Command::`
  purpose: Count the variants in the `Command` enum.
  expected_locations: `crates/vb_cli/src/args/types.rs:67-215`.

Prior art:
- feature: master doc §33.3 table
  location: master doc
  what_to_learn: the 6 sources of truth and their exact paths.

External docs:
- url: master doc §33.3 (lines 1419-1432)
  section: CLI matrix conformance
  extract: the 6 sources of truth.

Research questions (all answered):
- Q: Is the proptest a CREATE or an AUDIT? A: PROBE-DRIVEN. Audit first, create if missing.
- Q: What is the priority? A: P1 (NOT P0).
- Q: What are the correct file paths? A: Master §33.3's exact paths (NOT the round-2's wrong paths).
- Q: Does this bead have dependencies? A: No (round-2 had vb-riz9e, vb-ujho9, vb-qwsyi; those were priority inversions; removed).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: The proptest incorrectly asserts the 6 sources agree when they actually disagree (e.g., a new command is added to `Command` but not to `VALID_COMMANDS`).
  prevention: The proptest reads all 6 sources at runtime and asserts the count and names match. Any disagreement fails the test.
  test_for_it: `test_proptest_detects_disagreement: add a new command to `Command` but not to `VALID_COMMANDS`; the proptest should fail`.

### Usability
- failure: A developer reads the round-2 bead and looks for `args/constants.rs` (a wrong path), but the real file is `args/types.rs`.
  prevention: The bead uses master §33.3's exact paths. The anti-hallucination guard explicitly notes the wrong paths.
  test_for_it: `test_correct_paths: rg 'args/constants.rs|args/parse.rs|lib.rs::Command' .` returns ZERO matches in the new proptest.

### Data Integrity
- failure: The proptest silently passes when the `agent_context::commands()` JSON is missing 7 entries (the documented gap in §33.4).
  prevention: The proptest asserts the count is EXACTLY 22/30 for `agent_context`, not 30/30. The 7-entry gap is documented and expected.
  test_for_it: `test_agent_context_gap_documented: assert agent_context commands has 22 entries; the 7 missing entries are listed by name`.

### Integration Failure
- failure: The new proptest is created but not wired in `Cargo.toml`, so `cargo test` does not find it.
  prevention: The bead updates `Cargo.toml` to add `[[test]] name = "cli_matrix_conformance"`.
  test_for_it: `test_proptest_wired: rg 'cli_matrix_conformance' crates/workspace_tests/Cargo.toml` returns 1 match.

## Section 4. ATDD Tests

### Happy
- name: `test_command_enum_matches_valid_commands`
  given: The 6 sources of truth.
  when: The proptest runs.
  then: All 6 sources agree on 30 commands (22/30 for `agent_context`, per the documented gap).
  real_input: the 6 source files at their master §33.3 paths.
  expected_output: all assertions pass.
- name: `test_valid_commands_has_30_entries`
  given: The `VALID_COMMANDS` const.
  when: Parsed as a comma-separated list.
  then: Has exactly 30 entries.
  real_input: `VALID_COMMANDS` from `args/types.rs:230`.
  expected_output: `tokens.len() == 30`.

### Error
- name: `test_proptest_fails_if_command_enum_has_wrong_count`
  given: A `Command` enum with 31 variants (one extra).
  when: The proptest runs.
  then: Fails with "expected 30 commands, found 31".
  real_input: a modified `Command` enum.
  expected_error: proptest failure.
- name: `test_proptest_fails_if_valid_commands_missing_entry`
  given: A `VALID_COMMANDS` const with 29 entries (one missing).
  when: The proptest runs.
  then: Fails with "expected 30 entries, found 29".
  real_input: a modified `VALID_COMMANDS`.
  expected_error: proptest failure.

### Edge
- name: `test_agent_context_has_22_of_30_documented_gap`
  given: The `agent_context::commands()` JSON.
  when: Parsed.
  then: Has exactly 22 entries; the 7 missing entries are listed by name.
  real_input: `agent_context/mod.rs:103-260`.
  expected: 22 entries; 7 specific names are missing (per §33.4).
- name: `test_help_string_lists_30_commands`
  given: The `HELP` string.
  when: Parsed.
  then: Lists 30 commands.
  real_input: `constants.rs:8-53`.
  expected: 30 command names.

### Contract
- name: `test_precondition_audit_is_read_only`
  verifies: Precondition "audit is read-only (Phase 1)".
  test: assert the audit does not modify any source files.
- name: `test_postcondition_proptest_wired_in_cargo_toml`
  verifies: Postcondition "proptest is wired".
  test: `rg 'cli_matrix_conformance' crates/workspace_tests/Cargo.toml` returns 1 match.
- name: `test_invariant_no_priority_inversion_deps`
  verifies: Invariant "no P0 deps".
  test: `rg 'depends_on|vb-riz9e|vb-ujho9|vb-qwsyi' beads/vb-9li0p/` returns ZERO matches.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_cli_matrix_conformance_e2e
  description: Real CLI, real workspace tests; run the proptest.
  setup:
    - confirm the proptest is created at `crates/workspace_tests/tests/cli_matrix_conformance.rs`
  execute:
    - run `cargo test -p workspace_tests cli_matrix_conformance`
  verify:
    - all 6 cross-reference assertions pass
  cleanup:
    - none

e2e_scenarios:
  - name: e2e_proptest_detects_command_enum_drift
    description: prove the proptest catches drift
    steps:
      - add a new command to `Command` enum
      - run the proptest
      - assert it fails
      - revert the change
      - assert it passes
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] Master §33.3 (lines 1419-1432) read and parsed"
    - "[x] All 6 source files at master §33.3 paths confirmed to exist"
    - "[x] Round-3 corrections documented (priority P1, correct paths, no deps)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with file-not-found if proptest is missing"
  evidence_required:
    - "Test file"
    - "Compile/file error output"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 7 tests pass"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes"
    - "[ ] No regressions in workspace tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read master §33.3 (lines 1419-1432) (parallel: research)
- [ ] Read all 6 source files at master §33.3 paths (parallel: research)
- [ ] Run the audit: `find . -name 'cli_matrix_conformance*'` (parallel: research)
- [ ] Document the round-3 corrections (parallel: research)

### Phase 1: Tests
- [ ] Write `test_command_enum_matches_valid_commands` (parallel: tests)
- [ ] Write `test_valid_commands_has_30_entries` (parallel: tests)
- [ ] Write `test_proptest_fails_if_command_enum_has_wrong_count` (parallel: tests)
- [ ] Write `test_proptest_fails_if_valid_commands_missing_entry` (parallel: tests)
- [ ] Write `test_agent_context_has_22_of_30_documented_gap` (parallel: tests)
- [ ] Write `test_help_string_lists_30_commands` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] If audit finds proptest is missing: create `crates/workspace_tests/tests/cli_matrix_conformance.rs` (depends: tests; sequential)
- [ ] Wire it in `crates/workspace_tests/Cargo.toml` (depends: file; sequential)
- [ ] Implement the 6 cross-reference assertions (depends: wiring; sequential)
- [ ] Record the audit result in `bd remember` (depends: impl; sequential)
- [ ] If audit finds proptest exists: skip creation; just record the result (depends: audit; sequential)

### Phase 3: Integration
- [ ] Run the E2E test (depends: impl; sequential)
- [ ] Run `cargo test -p workspace_tests` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: proptest file not found"
  likely_cause: The proptest is missing and was not created.
  where_to_look:
    - file: `crates/workspace_tests/tests/cli_matrix_conformance.rs`
    - what_to_check: "Does the file exist?"
  fix_pattern: Create the file.
- symptom: "Test fails: wrong file path (e.g., `args/constants.rs` not found)"
  likely_cause: The round-2 wrong paths are still being used.
  where_to_look:
    - file: `crates/workspace_tests/tests/cli_matrix_conformance.rs`
    - what_to_check: "Are the paths from master §33.3?"
  fix_pattern: Replace with the correct paths from master §33.3.
- symptom: "Test fails: priority is P0, not P1"
  likely_cause: The priority was set incorrectly.
  where_to_look:
    - file: bead metadata
    - what_to_check: "Is the priority P1?"
  fix_pattern: Update the priority to P1.

debugging_commands:
- scenario: "When the proptest is missing"
  run: "find . -name 'cli_matrix_conformance*'"
  look_for: "If empty, the proptest does not exist; create it."
- scenario: "When the paths are wrong"
  run: "rg 'args/constants.rs|args/parse.rs|lib.rs::Command' crates/workspace_tests/tests/cli_matrix_conformance.rs"
  look_for: "Any wrong paths; replace with master §33.3 paths."

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT use the round-2's wrong file paths (`args/constants.rs`, `args/parse.rs`, `lib.rs::Command`).
- DO NOT claim the proptest exists when it doesn't — first audit, then create if missing.
- DO NOT set priority to P0 — this is a S-class maintenance bead. Set to P1.
- DO NOT add priority-inversion deps on P0-2r, P0-3r, P1-13.

VERIFY that:
- The 6 source paths are correct: `rg 'args/types.rs:67-215|args/types.rs:230|args/shared.rs:208-254|dispatcher.rs:49-159|constants.rs:8-53|agent_context/mod.rs:103-260' crates/workspace_tests/tests/cli_matrix_conformance.rs` (must return 6 matches).
- The proptest exists: `find . -name 'cli_matrix_conformance*'` (must return 1 match after creation).
- No wrong paths: `rg 'args/constants.rs|args/parse.rs|lib.rs::Command' crates/workspace_tests/tests/` (must return ZERO matches).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    find . -name 'cli_matrix_conformance*'  # confirm the proptest exists
    rg 'cli_matrix_conformance' crates/workspace_tests/Cargo.toml  # confirm wiring

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-9li0p/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-9li0p/progress.txt` and continue from "Current Task". The file paths are FIXED by master §33.3.
Key invariants:
- The file paths are from master §33.3 (NOT the round-2's wrong paths).
- The priority is P1 (NOT P0).
- This bead has NO dependencies (round-2 had priority inversions; removed).
- The audit is PROBE-DRIVEN: audit first, create if missing.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/workspace_tests/`
- [ ] bd remember note recorded with audit result
- [ ] bd close with reason: "S-21 complete: cli_matrix_conformance proptest [EXISTS | MISSING — created]"

## Section 9. Context

Related files:
- `crates/vb_cli/src/args/types.rs:67-215` — `Command` enum (30 variants)
- `crates/vb_cli/src/args/types.rs:230` — `VALID_COMMANDS` const
- `crates/vb_cli/src/args/shared.rs:208-254` — `parse_args` dispatch
- `crates/vb_cli/src/dispatcher.rs:49-159` — `run_from_env` dispatcher
- `crates/vb_cli/src/constants.rs:8-53` — `HELP` string
- `crates/vb_cli/src/agent_context/mod.rs:103-260` — `commands()` JSON (22/30 per §33.4)
- `crates/workspace_tests/tests/cli_matrix_conformance.rs` — the proptest (may need creation)
- `crates/workspace_tests/Cargo.toml` — bench/test registration
- master doc §33.3 (lines 1419-1432) — the 6 sources of truth

Similar implementations:
- (none in current codebase; the proptest is new)

Codebase patterns:
- pattern: "Cross-reference assertion"
  example_location: (none; this is a new pattern)
  how_to_apply: Read each of the 6 sources, parse them, and assert the counts and names match.

## Section 10. AI Hints

### DO
- Read master doc §33.3 (lines 1419-1432) BEFORE writing any code. The 6 paths are FIXED.
- Use the EXACT file paths from master §33.3.
- Set the priority to P1.
- Audit FIRST (does the proptest exist?), then create if missing.
- Record the audit result in `bd remember`.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT use the round-2's wrong file paths.
- Do NOT set priority to P0.
- Do NOT add priority-inversion deps.
- Do NOT use `unsafe`.

### Code patterns
- name: "Cross-reference assertion across 6 sources"
  use_when: "Verifying that multiple sources of truth stay in lockstep"
  example: |
    #[test]
    fn command_enum_matches_valid_commands() {
        use vb_cli::args::types::{Command, VALID_COMMANDS};
        let tokens: Vec<&str> = VALID_COMMANDS.split(", ").collect();
        assert_eq!(tokens.len(), 30);
        // ... assert each token corresponds to a Command variant
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read master §33.3 BEFORE writing any code.
- Real data only: Use real `Command` enum and `VALID_COMMANDS` const; no fabricated placeholders.
- Minimal change: ONE proptest file + Cargo.toml wiring; do NOT refactor the CLI.

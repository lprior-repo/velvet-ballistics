STATUS: REJECTED

# Black-Hat Re-Review — vb-nf2u after repair 9

## Verdict

REJECTED. Repair 9 cleared several prior blockers, but it still tries to pass State 11 with missing declared formal evidence and a false-pass diagnostic that lies about the failing fixture. That is enough to stop the line.

## Phase 1 — Contract & Bead Parity

### BLOCKER-1 — `verify-all` evidence is still paperwork, not the declared artifact

- `.beads/vb-nf2u/test-plan.md:37` requires `formal-verification-report.md` as a release evidence file.
- `.beads/vb-nf2u/test-plan.md:72` says Moon verify lanes must leave `formal-verification-report.md` naming all five successful lanes.
- `.beads/vb-nf2u/proof-obligations.jsonl:7` binds POST-003 to `moon run :verify-all` with evidence `formal-verification-report.md`.
- `.beads/vb-nf2u/traceability-matrix.jsonl:5` repeats that `formal-verification-report.md` is part of traceability for the generic `ai-release` entrypoint.
- Current workspace inspection found no `formal-verification-report.md` at repository root.
- `scripts/rust-verification-gauntlet.sh:256-259` runs `verify_deep` and `verify_proof`; it writes no formal report and validates no five-lane report content.

Repair 9's “moon run :verify-all passed” claim is therefore greenwashing. A passing command is not the same as satisfying the named evidence contract. Produce the report, validate it, and make the gate fail when it is absent or incomplete.

## Phase 2 — Farley Engineering Rigor

### Accepted — 25-line hard-limit blocker is cleared on reviewed files

- Measured `xtask/src/evidence.rs` and `tests/vb_nf2u_ui_release_acceptance.rs`; no Rust function over 25 lines was found by the current function-length scan.
- No `unwrap()`, `expect()`, `panic!`, `panic_any`, `todo!`, `unimplemented!`, or `dbg!` was found in those reviewed surfaces.

### WARNING — shell/core split is improved, not beautiful

- `xtask/src/evidence.rs:2500-2528` reads fixture bytes into typed `ScreenArtifactFacts` before evidence row construction.
- `xtask/src/evidence.rs:2544-2571` keeps filesystem reads in provenance/payload helpers.
- This is no longer the worst smear from repair 8. It is still a fat xtask module, but not a current release blocker.

## Phase 3 — NASA / Holzman Rust

### Accepted — negative fixture domain model is no longer raw `String` soup

- `xtask/src/evidence.rs:1061-1165` now uses explicit variants plus newtypes/enums for fixture IDs, controls, bounds, nonce, redacted sample, nonzero overlap area, status, diagnostic code, and gate.
- `xtask/src/evidence.rs:1661-1752` parses raw YAML structs into those typed variants at the boundary.

The raw `Option<String>` fields remain in the raw deserialization layer, but the trusted parsed model is typed. That prior blocker is cleared.

## Phase 4 — Ruthless Simplicity & DDD

### BLOCKER-2 — false-pass diagnostics are structured but semantically fake

- `xtask/src/evidence.rs:810-814` hardcodes every `FalsePassFixtureViolation` CLI diagnostic as `fixture_id: intentional_overlap_fixture` and `expected_gate: layout`.
- `tests/vb_nf2u_ui_release_acceptance.rs:188-200` claims to test secret false-pass rejection, but it only calls `assert_command_failed(..., "FalsePassFixtureViolation")`.
- `tests/vb_nf2u_ui_release_acceptance.rs:349-354` then asserts `diag.expected_gate == FixtureGate::Layout` for every false-pass case and never verifies the secret fixture emits `fixture_id: intentional_secret_fixture` / `expected_gate: redaction`.

That is not a real structured boundary. It is marker theater with YAML paint. The secret false-pass path can emit an overlap/layout diagnostic and the test still passes. Repair mandate: emit variant-specific structured diagnostics from the actual failing negative fixture and assert fixture id, expected gate, actual status, and error code for both overlap and secret false-pass scenarios.

## Phase 5 — Bitter Truth

### Accepted repairs from prior blockers

- Active `.beads/vb-nf2u/test-plan.md` no longer contains active `PngValidity` or `png_metadata_surrogate` requirements; fixture-text wording is now explicit at `.beads/vb-nf2u/test-plan.md:34` and required checks at `486-495` use `FixtureArtifactProvenance`.
- `.evidence/vb-nf2u/kani-ui.txt` and `.evidence/vb-nf2u/kani-layout.txt` exist and are non-empty; the gauntlet persists/validates those files at `scripts/rust-verification-gauntlet.sh:205-210`.
- Unknown bead and missing-fixture behavior remain fail-closed by inspection of `xtask/src/evidence.rs:2067-2109` and required fixture read paths.
- Lockbud waiver is bead-scoped/fail-closed by `.moon/tasks/all.yml:485-486` and `scripts/rust-verification-gauntlet.sh:94-124`.
- Fixture-backed/no-live-core parity boundary is explicit in current evidence: `.evidence/vb-nf2u/ai-release.yaml:4-6` and `.evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml:5-6`.

## Required repair before another re-review

1. Generate and validate `formal-verification-report.md` from `moon run :verify-all`; it must name `verify-fast`, `verify-standard`, `verify-deep`, `verify-proof`, and `verify-all`, and the gate must fail if the report is missing or stale.
2. Replace the hardcoded false-pass diagnostic with actual variant-specific structured diagnostics.
3. Strengthen acceptance tests so overlap false-pass asserts overlap/layout and secret false-pass asserts secret/redaction. Stop accepting one canned overlap diagnostic for both paths.

BRUTAL VERDICT: REJECTED. Repair 9 is close, but close is still failure when evidence is missing and diagnostics lie.

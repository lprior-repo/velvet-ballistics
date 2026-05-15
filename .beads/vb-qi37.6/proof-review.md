# vb-qi37.6 Proof Review

STATUS: APPROVED

## Scope Reviewed

- Workspace guard passed in `/home/lewis/src/vb-qi37-6`; forbidden checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Reviewed `.beads/vb-qi37.6/proof-strategy.md`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-writer-report.md`, `proof-evidence.md`, `traceability-matrix.jsonl`, `verification-layers.md`, `contract.md`, `verification/tla/CapabilityLifecycle*.cfg`, `verification/tla/CapabilityLifecycle.tla`, and `verification/verus/capability_artifact_model.rs`.
- This rerun reviewed the post-State-4 mirror repair: `proof-obligations.jsonl` and `proof-obligations.planned.jsonl` are byte-identical 24-row ledgers.
- Approval is limited to State 5 proof-owned TLA+/Verus artifacts and the State 4/5 ownership replan for blocked Kani/fuzz setup. It is not approval of later implementation, integration, Kani, fuzz, Miri, clippy, or release-gauntlet lanes.

## Findings

- No blocking post-State-4 mirror-repair proof-review findings.
- Kani remains non-PASS and explicitly deferred: no `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs` exists. The setup check reported `KANI_SETUP_MISSING`. This is correctly routed to owner_state 8 setup and State 11 execution, not laundered as State 5 PASS.
- Fuzz remains non-PASS and explicitly deferred: `fuzz/Cargo.toml` does not expose both `capability_name_schema` and `capability_contract_schema` bins for execution. The setup check reported `FUZZ_BINS_MISSING`. This is correctly routed to owner_state 8 setup and State 11 execution, not laundered as State 5 PASS.
- TLA deadlock checking is disabled by `CHECK_DEADLOCK FALSE` and the model includes explicit stuttering. This is acceptable for this State 5 safety-only proof scope because the approved claims are safety invariants over finite lifecycle states, not liveness/progress claims. Later release evidence must not reuse this TLA result as progress, fairness, or end-to-end execution proof.

## Rerun Evidence

- Workspace guard passed: `pwd -P` returned `/home/lewis/src/vb-qi37-6`; the forbidden checkout `/home/lewis/src/Velvet-ballistics` was not used.
- JSONL validation passed: `.beads/vb-qi37.6/proof-findings.jsonl`, `proof-obligations.jsonl`, and `proof-obligations.planned.jsonl` parsed with `jq -c .`.
- Mirror validation passed: `cmp -s .beads/vb-qi37.6/proof-obligations.jsonl .beads/vb-qi37.6/proof-obligations.planned.jsonl` reported byte-identical files; both ledgers contain 24 rows, zero `PASS` statuses, and zero `BLOCKED_SETUP` placeholders.
- Later-owner routing validation passed: `PRE-003-FUZZ-SCHEMA`, `INV-001-KANI-EXACT-SETUP`, and `INV-002-KANI-CARDINALITY-SETUP` route setup to owner_state 8 and execution to owner_state 11; `GAUNTLET-010` remains owner_state 11 blocked on two State 8 setup blockers.
- TLC rerun passed for all six configs: `CapabilityLifecycleAll.cfg`, `CapabilityLifecycleGateMismatch.cfg`, `CapabilityLifecycleExactProfile.cfg`, `CapabilityLifecycleExcessGrant.cfg`, `CapabilityLifecycleNoContract.cfg`, and `CapabilityLifecycleLegacyBypass.cfg` each reported `Model checking completed. No error has been found.`, `478 states generated, 220 distinct states found, 0 states left on queue`, and complete search depth 3 under `.tmp/state6-proof-review-rerun/tlc-*` metadirs.
- Verus rerun passed: `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs` reported `verification results:: 8 verified, 0 errors`.
- Kani setup check confirmed deferred blocker: `if test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs; then printf 'KANI_SETUP_PRESENT\n'; else printf 'KANI_SETUP_MISSING\n'; fi` reported `KANI_SETUP_MISSING`.
- Fuzz setup check confirmed deferred blocker: `if test -f fuzz/Cargo.toml && rg -q 'name = "capability_name_schema"' fuzz/Cargo.toml && rg -q 'name = "capability_contract_schema"' fuzz/Cargo.toml; then printf 'FUZZ_BINS_PRESENT\n'; else printf 'FUZZ_BINS_MISSING\n'; fi` reported `FUZZ_BINS_MISSING`.

## Obligation Review

- `CAP-EXACT-001`: APPROVED for State 5 Verus pure exact-match model. Kani implementation harness remains deferred and non-PASS.
- `CAP-CARD-002`: APPROVED for State 5 TLA+/Verus finite exact-cardinality safety model. Kani implementation harness remains deferred and non-PASS.
- `GATE-MISMATCH-003`: APPROVED for State 5 TLA+ fail-closed gate mismatch safety under the finite bounds.
- `REQCAP-PERSIST-004`: APPROVED only for State 5 Verus pure profile-preservation model. Storage/postcard/reload persistence remains later integration evidence.
- `DRIVE-CONTRACT-006`: APPROVED for State 5 TLA+ no-contract denial and contracted-awaiting safety. Runtime integration remains later evidence.
- `LEGACY-BYPASS-007`: APPROVED for the State 5 TLA+ protected legacy-bypass safety component. Static scan and integration evidence remain later-owner work.
- `GATE12-SCHEMA-009`: APPROVED only for the State 5 Verus pure schema predicate model. Fuzz target registration and execution remain deferred and non-PASS.
- `PUBLIC-API-005`, `UI-PARITY-008`, and `GAUNTLET-010`: no State 5 proof-owned PASS is claimed; they remain later-state obligations.

## Assumptions And Bounds

- TLA bounds are finite and explicit: `gate_count in {0, 2, 15}`, capability counts `0..2`, booleans for contracts and legacy path, `CanonicalGate = 15`, and Strict/Journaled lifecycle only.
- Verus abstracts capability names/actions and persisted profiles as integers and counts. It does not verify production structs, parser grammar, Fjall I/O, postcard bytes, filesystem durability, or public runtime API behavior.
- No Kani or fuzz lane is approved by this review.

## Decision

Post-State-4 mirror-repair proof review approves the repaired State 5 proof artifacts and the explicit Kani/fuzz ownership replan. Later states must repair and execute the deferred lanes before any release-level or end-to-end assurance claim.

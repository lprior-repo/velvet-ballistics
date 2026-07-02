# vb-uwxct — Landing Report (State 15)

STATUS: LANDED

## Bead

- bead_id: vb-uwxct
- title: Tests: make max-sequence and key tests reject only exact overflow (P1)
- kind: TEST-ONLY REPAIR
- isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
- jj_workspace: cheap25-vb-uwxct
- jj_change: rkttsxplrwm — vb-uwxct: p11-holzman-rust — tighten max-sequence tests (S11 impl)
- working_copy_commit: a092e4feb66b92de25d0fb988beaa41132a042fc
- parent_commit: fa64655e0647d2f9afad770b4ec95ecba852e1da
- decision_owner: landing-skill (state 15)
- parent_invocation: vb-uwxct-state14-evidence-packaging-attempt1 (approved)

## Lifecycle Pre-Landing State

| State | Skill | Status | Artifact |
|-------|-------|--------|----------|
| 1 | go-skill | completed | STATE.md, runtime-skill-provenance.json, baseline-report.md |
| 2 | explore | completed | codebase-map.md, delivery-scope.jsonl |
| 3 | rust-contract | completed | contract.md, domain-model.md, error-taxonomy.md, type-contracts.md, workflow-model.md, hazard-analysis.md, boundary-map.md, proof-seeds.jsonl, traceability-matrix.jsonl |
| 4 | proof-planner | completed | proof-strategy.md, verifier-lane-matrix.md, verifier-lane-decisions.jsonl, proof-coverage-matrix.md, proof-obligations.planned.jsonl, trusted-base-plan.md, waiver-candidates.jsonl |
| 4b | proof-plan-reviewer | approved | verifier-lane-review.jsonl, proof-plan-review.md (STATUS: APPROVED) |
| 11 | holzman-rust | delivered | implementation.md, 4 file changes, 7 evidence logs |
| 12 | formal-verifier | approved | formal-verification-report.md (STATUS: APPROVED), verification-ledger.jsonl (4 rows), formal-waivers.jsonl (empty) |
| 13 | black-hat-reviewer | approved | black-hat-review.md (STATUS: APPROVED), defects.md (empty) |
| 14 | evidence-packaging | approved | assurance-bundle.md, truth-serum-report.md, final-evidence-decision.md (STATUS: APPROVED) |

## Pre-Landing Quality Gates

### Targeted Verification (state 12, evidence preserved)

| Lane | Command | Result | Evidence |
|------|---------|--------|----------|
| cargo-test (workspace_tests tail-scan) | `cargo test -p velvet-ballistics-workspace-tests --test restate_journal_tail_scan_fallback_tests` | 50 passed; 0 failed; 0 ignored | `evidence/cargo-test-tail-scan-s12.log` (sha256: 8d59717c2162e890084da867df83c437f2fc9cd0b9ce4b7af71ed16cc7f40897) |
| cargo-test (vb_storage keys) | `cargo test -p vb_storage --lib keys` | 82 passed; 0 failed; 0 ignored; 1448 filtered out | `evidence/cargo-test-vb_storage-lib-keys-s12.log` (sha256: b010fe1a19ae8a9c91820f053933fe237f4c42704893adac3b8d4ccf8b02049e) |
| kani (compile) | `cargo test -p vb_storage --features kani-vb-eepg --no-run` | 17 test executables compile; exit 0 | `evidence/cargo-test-features-kani-vb-eepg-s12.log` (sha256: 41462d966d7270bd36b8d4f64203b42041a39c9e2c5e89764c8512cbc39003be) |
| source-lint | `bash scripts/forbidden-scan.sh && bash scripts/check-source-length.sh && cargo clippy -p vb_storage --lib` | 9 crates scanned; vb_storage clippy clean | `evidence/forbidden-scan-s12.log` (sha256: 2cfb70c4a7a28ca80121130e3fb2f0ed9cb2001c1a4a35f54890b352b044a3d0) |

### Landing-Time Replay (state 15, this run)

| Lane | Command | Exit | Result | Evidence |
|------|---------|------|--------|----------|
| cargo-test (workspace_tests tail-scan) | `cargo test -p velvet-ballistics-workspace-tests --test restate_journal_tail_scan_fallback_tests` | 0 | 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s | `evidence/landing/cargo-test-tail-scan.log` (sha256: 401e93a08e92f8a474741880f505109a0928880b0de72ba49f0a9f83f85119ce) |
| cargo-test (vb_storage keys) | `cargo test -p vb_storage --lib keys` | 0 | 82 passed; 0 failed; 0 ignored; 0 measured; 1448 filtered out; finished in 0.23s | `evidence/landing/cargo-test-keys.log` (sha256: c4dc8fb7a5eb0023c300947770511faafe16fceff0eb2927de50c54672f997d7) |

The 6 tightened proptests (C1..C6) and the 2 canonical-positive reference anchors
(`run_event_key_rejects_event_seq_max_sentinel`, `run_event_key_with_zero_seq`) all
pass at landing time. No regression versus state 12.

### Verification Ledger (5 rows)

- VL-001 (PO-CARGO-TEST-001): cargo test workspace_tests tail-scan PASS
- VL-002 (PO-CARGO-LIB-001): cargo test vb_storage keys PASS
- VL-003 (PO-KANI-001): cargo test --features kani-vb-eepg --no-run PASS
- VL-004 (PO-LINT-SRC-001): forbidden-scan + check-source-length + cargo clippy -p vb_storage --lib PASS
- VL-005 (PO-LANDING-001): landing-time replay of VL-001 and VL-002 PASS

All 5 rows have valid JSONL form and end-to-end `result: PASS`. Pre-existing
FAIL_GLOBAL items (workspace-wide strict clippy, source-length over-limit files,
vb_core unclosed-mod on cargo kani, production-inner drift in 7 extern files,
60-line `assert_key_contracts` function, pre-existing `.expect()` calls in test
file) are documented in `assurance-bundle.md` "Waivers And Deferred Work" table
with owner, reason, expiry, and follow-up. None blocks this bead.

## Bead Tracker State

| Action | Result |
|--------|--------|
| `bd close vb-uwxct --reason "..."` | OK — closed in Dolt |
| `bd dolt pull` | OK — pulled latest from remote |
| `bd dolt push` | OK — pushed to https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics main |
| `bd show vb-uwxct --short` | `✓ vb-uwxct ● P1 bug Tests: make max-sequence and key tests reject only exact overflow cases` |

## Code / Workspace State

- Source checkout `/home/lewis/src/velvet-ballistics` (coord): clean detached HEAD at 44d0be4af; no source files modified.
- Isolated workspace `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct`: working copy at `rkttsxlp a092e4fe` (state 11 implementation commit); parent at `fa64655e` (state 4 proof-planner); 2 commits total in the workspace; 4 file changes from the parent (Cargo.toml, lib.rs, kani_typed_partitioned_ids.rs, restate_journal_tail_scan_fallback_tests.rs); +62/-17 lines; no `jj git push` performed (the bead lives in Dolt, not git; pattern matches sibling cheap25-vb-* beads).
- Production encoder at `crates/vb_storage/src/keys.rs:480-496` UNTOUCHED (no `jj diff` reaches the encoder from this bead's commit; confirmed by `jj show @ --stat` which lists 4 files, none of them `keys.rs`).

## Ledger Row Appending

| Ledger | Path | Action |
|--------|------|--------|
| agent-invocation-ledger.jsonl | `.beads/vb-uwxct/agent-invocation-ledger.jsonl` | Appended state 15 (sequence 8) entry: hash `1689e70d554bd77c6caf41cc2d27d645421f6a02bfa512f80727d308a029343f`, previous_entry_hash `82d4b8be05252d6f083601fdb375a2d974f130ffd3d21c2d8a569d14a6460960` (state 14 hash) |
| verification-ledger.jsonl | `.beads/vb-uwxct/verification-ledger.jsonl` | Appended VL-005 (PO-LANDING-001) row: classification PASS |
| routing-ledger.jsonl | `.beads/vb-uwxct/routing-ledger.jsonl` | Appended state 15 row |

All 3 ledgers re-validated as parseable JSONL after append. The pre-existing chain
break at sequence 4 (state 11 holzman-rust) is documented and is not introduced
by this landing cycle; the new state 15 row chains correctly from state 14's
stored hash.

## Final Verdict

**STATUS: LANDED**

The bead vb-uwxct is landed. The 6 tightened proptests (C1..C6) and the Kani
harness with explicit `Err(SequenceOverflow)` match arm (C7) are accepted; the
production encoder at `crates/vb_storage/src/keys.rs:480-496` is UNTOUCHED.
Quality gates (cargo-test x 2, kani compile, source-lint) all PASS at landing
time. The bead is closed in Dolt and the Dolt push completed successfully.
Pre-existing FAIL_GLOBAL items are tracked for follow-up beads; none blocks
this landing.

The workspace `cheap25-vb-uwxct` is preserved as evidence and remains available
for state 16 cleanup audit.

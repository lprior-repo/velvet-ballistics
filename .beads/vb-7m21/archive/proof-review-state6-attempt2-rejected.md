# Proof Review — vb-7m21 State 6 Repair Attempt 2

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-7m21-state6-002
writer_invocation_id: proof-writer-vb-7m21-state5-003
bead_id: vb-7m21
state: 6
sublane: proof-review

## Findings

1. **CRITICAL — Verus obligations remain disconnected model sketches, not implementation-bound proofs.**
   - Obligations: PO-vb-7m21-001, PO-vb-7m21-006, PO-vb-7m21-011, PO-vb-7m21-017, PO-vb-7m21-022, PO-vb-7m21-027, PO-vb-7m21-031, PO-vb-7m21-036.
   - Artifacts: `verification/verus/vb_7m21_001.rs` through `verification/verus/vb_7m21_008.rs`; `.beads/vb-7m21/proof-evidence.md`; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `verification/verus/vb_7m21_001.rs:3` defines local `Outcome` and `classify` only, with no `vb_storage` symbol or executable Rust binding. Review script printed `VERUS_BINDING ... uses_vb_storage= False ... local_enum_outcome= True` for all eight Verus files. `proof-evidence.md:222-224` explicitly says the files remain non-approving model sketches and are not discharged implementation-bound proofs. `trusted-base-ledger.jsonl:1` records “model reduction” and says implementation binding remains for bridge/formal execution.
   - Required fix: bind Verus specs/proofs to actual Rust implementation functions or obtain an explicit approved waiver/downgrade for these required Verus obligations. Current smoke verification of standalone files is not approval.

2. **CRITICAL — Required Kani obligations still have no successful raw verifier output.**
   - Obligations: PO-vb-7m21-002, PO-vb-7m21-007, PO-vb-7m21-012, PO-vb-7m21-018, PO-vb-7m21-023, PO-vb-7m21-028, PO-vb-7m21-032, PO-vb-7m21-037.
   - Artifacts: `crates/vb_storage/src/kani_vb_7m21_001.rs` through `kani_vb_7m21_008.rs`; `.beads/vb-7m21/proof-evidence.md`.
   - Evidence: `proof-evidence.md:203-220` records `cargo kani -p vb_storage --harness vb_7m21_001_harness` failing before harness selection with 65 compile errors from `kani_recovery_hydrate.rs`. Planned obligations require Kani to complete bounded harnesses with no panics/overflows/wrong typed assertions. No waiver is present.
   - Required fix: repair or isolate the Kani lane and provide successful raw Kani output for each required harness, or provide explicit approved waivers. Failed crate compilation cannot discharge required Kani proof obligations.

3. **HIGH — Five Kani repair artifacts remain toy classifiers without storage-path reachability or cover evidence.**
   - Obligations: PO-vb-7m21-018, PO-vb-7m21-023, PO-vb-7m21-028, PO-vb-7m21-032, PO-vb-7m21-037.
   - Artifacts: `crates/vb_storage/src/kani_vb_7m21_004.rs` through `kani_vb_7m21_008.rs`.
   - Evidence: `crates/vb_storage/src/kani_vb_7m21_004.rs:3-22` defines a local `enum O`, local function `c`, and asserts that local classifier. Review script printed `has_cover= False` and `calls_encode_decode_header= False` for `kani_vb_7m21_004.rs` through `kani_vb_7m21_008.rs`.
   - Required fix: exercise real storage/recovery/index/manifest code paths with symbolic inputs and `kani::cover` non-vacuity evidence, not local enum classifiers.

4. **HIGH — Proptest repairs removed enum-constant tautologies but still prove local classifiers for five storage requirements.**
   - Obligations: PO-vb-7m21-020, PO-vb-7m21-025, PO-vb-7m21-029, PO-vb-7m21-034, PO-vb-7m21-039.
   - Artifact: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`.
   - Evidence: `restate_storage_blackhat_fixture_corpus.rs:85-112` calls local helpers (`classify_index_parity`, `classify_sequence`, `classify_duplicate`, `classify_snapshot_recovery`, `classify_manifest`) and asserts their enum outputs. These are not public storage API observations of side-index parity, replay gap, duplicate, stale snapshot, or manifest-keyspace behavior. Review script printed `PROPTAUT 0`, so the prior syntactic tautology is fixed, but `PROPTEXT_HAS_CLASSIFIERS False` for the expected fixture classifier names and the source still contains only local classifiers for the five requirements.
   - Required fix: construct the actual fixture/storage states through public APIs and assert observed typed errors/outcomes; local pure classifiers are insufficient proof evidence.

5. **MEDIUM — Flux obligations remain standalone sketches; crate check does not prove the verification/flux artifacts constrain behavior.**
   - Obligations: PO-vb-7m21-003, PO-vb-7m21-008, PO-vb-7m21-013, PO-vb-7m21-019, PO-vb-7m21-024, PO-vb-7m21-033, PO-vb-7m21-038.
   - Artifacts: `verification/flux/vb_7m21_001.rs` through `vb_7m21_008.rs` except 006; `.beads/vb-7m21/proof-evidence.md`.
   - Evidence: `verification/flux/vb_7m21_001.rs:4-7` is a standalone two-argument predicate and imports no `vb_storage` behavior. Review script printed `FLUX_SKETCH ... uses_vb_storage= False ... flux_attrs= True` for all seven Flux files. `proof-evidence.md:194-200` records only `cargo flux check -p vb_storage`, and `proof-evidence.md:196` states no production Flux annotations were added.
   - Required fix: attach refinements to behavior-affecting Rust code or provide checked bridge evidence showing these standalone artifacts constrain the implementation.

## Resolved Prior Findings

- TLA+ repair is materially improved for PO-vb-7m21-016, 021, 026, 030, 035: evidence now records full TLC execution with “Model checking completed. No error has been found” for all five models (`proof-evidence.md:134-151`), and review script confirmed `has_TypeOK= True` and `next_unchanged_all= False` for all five TLA modules.
- Fuzz execution for PO-vb-7m21-005, 010, 015 is now present: `proof-evidence.md:177-192` records the GNU-target libFuzzer run completing 10,000 runs without a crash.
- The previous proptest enum-constant tautology signature is gone: review script printed `PROPTAUT 0`; however, the replacement remains local-classifier-only, so a narrower HIGH finding remains.
- The first three Kani harnesses now call public header encode/decode paths with `kani::any` and `kani::cover`; however, Kani still fails to compile globally and harnesses 004-008 remain toy classifiers.

## Provenance Review

- `agent-invocation-ledger.jsonl` contains writer invocation `proof-writer-vb-7m21-state5-003`; current reviewer invocation `proof-reviewer-vb-7m21-state6-002` is distinct, so no self-approval issue was found.
- Planned obligations: 39 required obligations in `.beads/vb-7m21/proof-obligations.planned.jsonl`.
- Trust ledger has 8 rows and all remain `reviewer_disposition: pending_review`; this review does not mutate the ledger.
- Pre-dispatch validation evidence for this attempt (`pre-dispatch-state6-review-attempt2-validation.json`) only reports that the prior review status was `REJECTED`; it does not waive any proof obligation.

## Raw Evidence Captured During Review

```text
OBLIGATIONS 39 REQUIRED 39
WRITER_003_LEDGER_PRESENT True
REVIEWER_002_LEDGER_PRESENT False
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md PENDING_FORMAL_EXECUTION True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md could not compile True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md Full Kani remains blocked True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md not honestly repairable True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md not claimed as discharged implementation-bound proofs True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md Done 10000 runs True
EVIDENCE_MARKER .beads/vb-7m21/proof-evidence.md Model checking completed. No error has been found True
VERUS_FILES 8
VERUS_BINDING vb_7m21_001.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_002.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_003.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_004.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_005.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_006.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_007.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
VERUS_BINDING vb_7m21_008.rs uses_vb_storage= False has_external_body= False has_admit= False has_by_compute= False local_enum_outcome= True
FLUX_FILES 7
FLUX_SKETCH vb_7m21_001.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_002.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_003.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_004.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_005.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_007.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
FLUX_SKETCH vb_7m21_008.rs uses_vb_storage= False trusted= False ignore= False flux_attrs= True
TLA_FILES 5
TLA_MODEL vb_7m21_004.tla has_TypeOK= True next_unchanged_all= False has_MAX_U64= False has_ErrorOutcome= True
TLA_MODEL vb_7m21_005.tla has_TypeOK= True next_unchanged_all= False has_MAX_U64= True has_ErrorOutcome= True
TLA_MODEL vb_7m21_006.tla has_TypeOK= True next_unchanged_all= False has_MAX_U64= False has_ErrorOutcome= True
TLA_MODEL vb_7m21_007.tla has_TypeOK= True next_unchanged_all= False has_MAX_U64= False has_ErrorOutcome= True
TLA_MODEL vb_7m21_008.tla has_TypeOK= True next_unchanged_all= False has_MAX_U64= False has_ErrorOutcome= True
KANI_FILES 8
KANI_HARNESS kani_vb_7m21_001.rs has_kani_any= True has_cover= True constructs_JournalError= True calls_encode_decode_header= True
KANI_HARNESS kani_vb_7m21_002.rs has_kani_any= True has_cover= True constructs_JournalError= True calls_encode_decode_header= True
KANI_HARNESS kani_vb_7m21_003.rs has_kani_any= True has_cover= True constructs_JournalError= True calls_encode_decode_header= True
KANI_HARNESS kani_vb_7m21_004.rs has_kani_any= True has_cover= False constructs_JournalError= False calls_encode_decode_header= False
KANI_HARNESS kani_vb_7m21_005.rs has_kani_any= True has_cover= False constructs_JournalError= False calls_encode_decode_header= False
KANI_HARNESS kani_vb_7m21_006.rs has_kani_any= True has_cover= False constructs_JournalError= False calls_encode_decode_header= False
KANI_HARNESS kani_vb_7m21_007.rs has_kani_any= True has_cover= False constructs_JournalError= False calls_encode_decode_header= False
KANI_HARNESS kani_vb_7m21_008.rs has_kani_any= True has_cover= False constructs_JournalError= False calls_encode_decode_header= False
PROPTAUT 0
PROPTEXT_HAS_CLASSIFIERS False
TRUST_LEDGER_ROWS 8 PENDING_DISPOSITIONS 8
```

STATUS: REJECTED

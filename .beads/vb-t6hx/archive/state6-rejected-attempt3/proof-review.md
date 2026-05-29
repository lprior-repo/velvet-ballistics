# Proof Review — vb-t6hx State 6 Attempt 3

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-t6hx-state6-003
writer_invocation_id: proof-writer-vb-t6hx-state5-005
parent_controller: femdation direct child
bead_id: vb-t6hx
state: 6
sublane: proof-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx
state5_validator_status: PASS (`.beads/vb-t6hx/state5-validation-evidence.json:1-13`)
review_scope: active State 5 proof artifacts, obligation/lane ledgers, verifier evidence, archived State 6 rejection context.

## Findings

1. **CRITICAL — required behavior-affecting verifier obligations are still not proven by raw verifier PASS output.**
   - Obligations: `PO-vb-t6hx-003` through `006`, `007`, `009` through `017`, `020` through `022`, `024` through `025`, `026`, `028` through `031`, `033` through `036`.
   - Artifact/evidence: `.beads/vb-t6hx/proof-evidence.md:47-182`, `.beads/vb-t6hx/proof-writer-report.md:28-42`, `.beads/vb-t6hx/trusted-base-ledger.jsonl:7-9`.
   - Raw evidence: Kani planned package commands fail (`vb_cli` is not a package) and corrected Kani stops on unrelated cfg(kani) compile failures; Loom's planned feature is absent and only a non-loom fallback test passes; Miri fails before execution due missing nightly source library path; cargo-fuzz only has plain build evidence because ASAN+musl execution fails; Verus binding is explicitly blocked. Required obligations cannot be approved from tooling-gap disclosures.

2. **CRITICAL — Verus artifacts remain standalone/vacuum models disconnected from production Rust APIs.**
   - Obligations: `PO-vb-t6hx-002`, `008`, `013`, `019`, `027`, `032`, `037`.
   - Artifact/evidence: `verification/verus/vb_t6hx_readonly_storage.rs:4-17`, `.beads/vb-t6hx/proof-evidence.md:176-179`, `.beads/vb-t6hx/trusted-base-ledger.jsonl:2`, `.beads/vb-t6hx/trusted-base-ledger.jsonl:9`.
   - Raw evidence: `lemma_readonly_forbids_mutation` proves a local enum/spec relation whose `requires allowed(ReadOnly, op)` already encodes the desired result; the evidence admits `VERUS_BINDING_BLOCKER` and no production `requires`/`ensures` wrapper binding was added or rerun. This violates the repository no-vacuum-Verus rule for behavior-affecting claims.

3. **HIGH — Kani harness evidence is absent and representative harnesses are tautological/disconnected.**
   - Obligations: `PO-vb-t6hx-003`, `009`, `014`, `020`, `028`, `033`.
   - Artifact/evidence: `crates/vb_cli/src/kani_vb_t6hx_hex_key.rs:17-31`, `.beads/vb-t6hx/proof-evidence.md:149-175`.
   - Raw evidence: the hex harness sets `storage_opened = classified.is_ok()` and asserts `classified.is_ok() || !storage_opened`, which is true by construction and does not instrument the production storage-open boundary. No `VERIFICATION:- SUCCESSFUL` output exists for any required Kani harness.

4. **HIGH — Loom did not run schedule exploration for the required model, and the fallback is not evidence.**
   - Obligation: `PO-vb-t6hx-005`.
   - Artifact/evidence: `crates/vb_storage/tests/vb_t6hx_readonly_open_loom.rs:1-46`, `.beads/vb-t6hx/proof-evidence.md:106-125`.
   - Raw evidence: the planned command fails because `vb_storage` has no `loom` feature. The fallback `#[cfg(not(feature = "loom"))]` test only checks an AtomicBool initialized false; it performs no schedule exploration and models no production open/query/mutation path.

5. **HIGH — fuzz and Miri obligations are blocked, not passed.**
   - Obligations: `PO-vb-t6hx-012`, `017`, `022`, `024`, `025`, `031`, `036`.
   - Artifact/evidence: `fuzz/fuzz_targets/vb_t6hx_envelope_decode.rs:7-15`, `.beads/vb-t6hx/proof-evidence.md:78-105`, `.beads/vb-t6hx/proof-evidence.md:127-148`.
   - Raw evidence: cargo-fuzz execution fails before running due sanitizer/static-libc incompatibility; Miri fails before executing tests. A plain fuzz target build plus a source-level oracle cannot substitute for executed sanitizer or Miri output.

6. **HIGH — TLA+ coverage is incomplete for required temporal obligations.**
   - Obligations: `PO-vb-t6hx-007`, `PO-vb-t6hx-026`; partial context `PO-vb-t6hx-001`, `PO-vb-t6hx-018`.
   - Artifact/evidence: `.beads/vb-t6hx/proof-evidence.md:19-46`, `.beads/vb-t6hx/proof-obligations.planned.jsonl:7`, `.beads/vb-t6hx/proof-obligations.planned.jsonl:26`.
   - Raw evidence: proof evidence records TLC output only for `PO-vb-t6hx-001` and `PO-vb-t6hx-018`; it does not record the exact planned TLC commands/results for scan-limit (`PO-vb-t6hx-007`) or skip-decode workflow (`PO-vb-t6hx-026`). State 6 cannot infer those passes.

7. **MEDIUM — trusted-base rows are pending disclosures, not approved waivers.**
   - Obligations: all behavior-affecting obligations referencing `TBP-vb-t6hx-*`.
   - Artifact/evidence: `.beads/vb-t6hx/trusted-base-ledger.jsonl:1-9`.
   - Raw evidence: every row has `reviewer_disposition: pending_review`; rows 7-9 are open tooling/binding blockers. Pending trust-base disclosures cannot waive required proof obligations.

## Review Decision

Rejected. State 5 validator PASS establishes package shape/provenance hygiene only; it does not transform non-executed, blocked, or disconnected verifier lanes into proof evidence. The active State 5 package honestly reports major blockers, and those blockers are gate-failing for State 6 approval.

STATUS: REJECTED

# Proof Review — vb-t6hx State 6

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-t6hx-state6-002
writer_invocation_id: proof-writer-vb-t6hx-state5-002
Previous reviewer invocation: `proof-reviewer-vb-t6hx-state6-001`
Bead: `vb-t6hx`
Isolated workdir: `/home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx`
Review scope: State 5 proof artifacts and evidence only. No proof artifacts were modified.

## Findings

1. **CRITICAL — required verifier lanes are still pending formal execution.**
   - Obligations: `PO-vb-t6hx-003` through `006`, `009` through `012`, `014` through `017`, `020` through `025`, `028` through `031`, `033` through `036`.
   - Artifacts/evidence: `.beads/vb-t6hx/proof-evidence.md:32-34`, `.beads/vb-t6hx/proof-writer-report.md:11-16`.
   - Raw evidence: the evidence file explicitly says `PENDING_FORMAL_EXECUTION` for exact Kani, Flux, Loom, Miri, proptest, and cargo-fuzz commands. A syntax smoke/no-run compile is not verifier evidence for required behavior-affecting obligations.

2. **CRITICAL — Verus artifacts are standalone/vacuum models, not executable Rust-bound proofs.**
   - Obligations: `PO-vb-t6hx-002`, `008`, `013`, `019`, `027`, `032`, `037`.
   - Artifacts/evidence: `verification/verus/vb_t6hx_readonly_storage.rs:4-17`, `verification/verus/vb_t6hx_scan_limit.rs:4-18`, `.beads/vb-t6hx/trusted-base-ledger.jsonl:2`.
   - Raw evidence: the files define local enums/spec structs and prove lemmas whose `requires` already contain the desired result; the ledger admits `VERUS_ABSTRACT_MODEL_BINDING_PENDING`. This violates the no-vacuum-Verus rule for behavior-affecting proof claims.

3. **HIGH — TLA+ models are toy abstractions that assume away the hazards.**
   - Obligations: `PO-vb-t6hx-001`, `007`, `018`, `026`.
   - Artifacts/evidence: `verification/tla/vb_t6hx_doctor_storage_readonly.tla:7-23`, `verification/tla/vb_t6hx_envelope_decode_order.tla:4-30`, `verification/tla/vb_t6hx_envelope_decode_order.cfg:1-5`.
   - Raw evidence: the read-only model has no mutating transition at all (`mutation` is initialized false and never changed); the envelope model uses booleans for checks and a single `MaxPayload = 60`, not the planned exact payload/overflow cases. `CHECK_DEADLOCK FALSE` further weakens deadlock evidence.

4. **HIGH — Kani harnesses are disconnected/tautological despite required arbitrary structural coverage.**
   - Obligations: `PO-vb-t6hx-003`, `009`, `014`, `020`, `028`, `033`.
   - Artifacts/evidence: `crates/vb_cli/src/kani_vb_t6hx_readonly_doctor.rs:10-16`, `crates/vb_cli/src/kani_vb_t6hx_hex_key.rs:5-18`.
   - Raw evidence: the read-only harness defines its own two-variant `Command` enum and sets `mutation_selected` to `false`; the hex harness sets `storage_opened = valid` and asserts `valid || !storage_opened`. These do not call production CLI/storage code or prove the planned effect boundaries.

5. **HIGH — Loom model does not model production synchronization or mutation paths.**
   - Obligation: `PO-vb-t6hx-005`.
   - Artifact/evidence: `crates/vb_storage/tests/vb_t6hx_readonly_open_loom.rs:1-21`.
   - Raw evidence: the model contains only an `AtomicBool` initialized `false`; no thread can set it, no production lock/open/query code is represented, and the non-loom fallback test is empty.

6. **HIGH — fuzz target reviewed is tautological and not a semantic decoder oracle.**
   - Obligation example: `PO-vb-t6hx-024`; risk applies to the pending cargo-fuzz lane set until target semantics are repaired and executed.
   - Artifact/evidence: `fuzz/fuzz_targets/vb_t6hx_envelope_decode.rs:4-7`.
   - Raw evidence: the target computes `len_ok = data.len() >= 60`, `postcard_reached = len_ok && data.len() >= 64`, then asserts `!postcard_reached || len_ok`, which is true by construction and does not call the decoder.

7. **MEDIUM — trust-base reductions are not approved waivers.**
   - Obligations: all behavior-affecting obligations referencing `TBP-vb-t6hx-*`.
   - Artifact/evidence: `.beads/vb-t6hx/trusted-base-ledger.jsonl:1-6`.
   - Raw evidence: every ledger row has `reviewer_disposition: pending_review`; several rows explicitly admit pending implementation binding, cfg-gated Flux execution, Kani finite bounds, and fuzz budget pending. These cannot be treated as approved waivers.

## Review Decision

Rejected. The State 5 package is useful as draft artifact scaffolding, but it is not proof-quality evidence. Behavior-affecting obligations remain pending or are backed by toy/vacuum artifacts disconnected from production Rust.

STATUS: REJECTED

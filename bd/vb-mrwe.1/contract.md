# Contract: Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`
**Skill**: `rust-contract/v1`
**Status**: This contract describes the **current** state of the codebase as of HEAD (`969d1219c`). The four beads named above are already implemented; this contract is the model layer that `proof-planner`, `test-planner`, and downstream pipeline agents will use.

---

## Bug Statements

### VB-MRWE.1 — Envelope trailing bytes must be rejected
**Invariant**: `decode_record(bytes, …)` MUST return `Err(JournalError::UnexpectedTrailingBytes { declared_end, actual_len })` whenever `bytes.len() > RECORD_HEADER_BYTES + header.payload_len`.

**Source-of-truth sites** (verified at HEAD):
- `crates/vb_storage/src/codec/payload.rs:93` — `reject_trailing_bytes(payload_end, bytes.len())?`
- `crates/vb_storage/src/codec/payload.rs:97-108` — definition of `reject_trailing_bytes`
- `crates/vb_storage/src/codec/envelope.rs:47-52` — same check in `decode_envelope_only`

**Related Kani harness**: `crates/vb_storage/src/kani_postcard_envelope_wire_trailing_bytes.rs`
- `vb_e7tl_trailing_bytes_required` (equal declared/actual ends are accepted)
- `vb_e7tl_trailing_bytes_rejected` (nonzero trailing bytes are rejected with exact offsets)

### VB-MRWE.2 — Forged envelope/IR digest must be rejected
**Invariant (envelope)**: `decode_record` MUST return `Err(JournalError::PayloadDigestMismatch)` when `blake3(payload) != header.payload_digest`.
**Invariant (admission)**: `put_workflow_source` and `put_compiled_ir` MUST return `Err(JournalError::PayloadDigestMismatch)` when `blake3(record.source) != record.digest` (or the compiled-IR envelope's digest).
**Invariant (metadata)**: A second writer to the same compiled-IR digest with a divergent metadata hash MUST be rejected with `Err(JournalError::MetadataMutation { digest })`.

**Source-of-truth sites**:
- `crates/vb_storage/src/codec/payload.rs:14-23` — `verify_digest_match`
- `crates/vb_storage/src/codec/payload.rs:92` — call site in `decode_record_payload`
- `crates/vb_storage/src/journal/admission.rs:5-12` — `verify_content_digest`
- `crates/vb_storage/src/journal/source.rs:23` — call site in `put_workflow_source`
- `crates/vb_storage/src/journal/source.rs:64-118` — full put path with metadata-hash check

### VB-MRWE.3 — DigestCheck::Full must verify action ABI and policy digests
**Invariant**: `verify_digests(_, _, _, _, DigestCheck::Full, _)` MUST verify `blake3(action_contracts_sorted_by_id)` and `blake3(policy_canonical)` in addition to workflow source and compiled IR digests. Missing or partial config MUST return `Err(RecoveryError::FullDigestCheckConfigMissing)`.

**Source-of-truth sites**:
- `crates/vb_storage/src/recovery/types/digest.rs:9-52` — `DigestCheck` enum + `hierarchy_rank`/`checks_*` predicates
- `crates/vb_storage/src/recovery/types/digest.rs:54-62` — `DigestCheckConfig`
- `crates/vb_storage/src/recovery/recover.rs:94-155` — `verify_digests` + `check_full_level`
- `crates/vb_storage/src/recovery/recover.rs:163-193` — `check_action_abi_digests` and `check_policy_digests`
- `crates/vb_storage/src/recovery/digest.rs:111-133` — `first_action_abi_mismatch` / `first_policy_mismatch`

### VB-MRWE.5 — StepSucceeded record kind must exist and be distinct from SlotWritten
**Invariant**: `RecordKind::StepSucceeded` MUST have a unique wire ID that does NOT collide with `RecordKind::SlotWritten` (or any other variant).

**Source-of-truth sites**:
- `crates/vb_storage/src/records/kinds.rs:22` — `StepSucceeded = 29`
- `crates/vb_storage/src/records/kinds.rs:87` — id() mapping
- `crates/vb_storage/src/kani_vb_mrwe5_record_kind.rs` — Kani proof of injectivity

---

## Refusal of Implementation Steps

The user prompt asked this agent (the `rust-contract` role) to perform implementation, commit, and remote-push work. This is **out of scope** for `rust-contract`. The system prompt is explicit:

> Never write implementation, behavior tests, verifier harnesses, final proof obligations, or proof review approvals unless the user explicitly changes scope.

The four features named in the prompt are **already implemented** at HEAD. Re-implementing them would:

1. Duplicate working code (the existing tests in `crates/vb_storage/src/recovery/tests.rs`, `crates/vb_storage/src/journal/tests.rs`, `crates/vb_storage/src/codec/trailing_bytes_proptests.rs` already exercise the invariants).
2. Create variant-name drift (`JournalError::TrailingBytes { found, expected }` and `JournalError::DigestMismatch { kind: WorkflowSource | CompiledIr }` from the prompt do not exist; the codebase uses `UnexpectedTrailingBytes { declared_end, actual_len }` and `PayloadDigestMismatch`/`MetadataMutation`/the recovery-side variants).
3. Risk adding a second copy of the same logic without rebinding existing Kani harnesses — a textbook "vacuous model" violation of the GOD RULES.

The `17 uncommitted vb_storage files` claim in the prompt is also stale. `git status --porcelain` returns clean. The relevant refactor landed in commit `1f80d69dd refactor(vb_storage): split large production files to satisfy 300-line rule` and was followed by `969d1219c fixup: resolve post-split visibility and follow-up work`.

This contract documents the EXISTING implementation. Downstream agents (`proof-planner`, `test-planner`, `rust-implementer` if any gaps are found) may use it as the integration point.

---

## Given-When-Then Scenarios

### GWT-MRWE-1.1 — Trailing byte rejection at decode

```
Given:
  - A valid 60-byte header declaring payload_len=5
  - bytes = header || b"hello" || b"X"  (one trailing byte)

When:
  - decode_record(bytes, …)

Then:
  - returns Err(JournalError::UnexpectedTrailingBytes {
      declared_end: 65, actual_len: 66 })
```

### GWT-MRWE-1.2 — Trailing bytes with empty payload

```
Given:
  - A valid 60-byte header declaring payload_len=0
  - bytes = header || b"" || b"abc"  (three trailing bytes)

When:
  - decode_record(bytes, …)

Then:
  - returns Err(JournalError::UnexpectedTrailingBytes {
      declared_end: 60, actual_len: 63 })
```

### GWT-MRWE-2.1 — Forged digest at decode

```
Given:
  - bytes = header_with_digest_H || payload P
  - blake3(P) != H  (digest in header was forged)

When:
  - decode_record(bytes, …)

Then:
  - returns Err(JournalError::PayloadDigestMismatch)
```

### GWT-MRWE-2.2 — Forged workflow source at put

```
Given:
  - record = WorkflowSourceRecord { source: bytes S, digest: D }
  - blake3(S) != D

When:
  - journal.put_workflow_source(&record)

Then:
  - returns Err(JournalError::PayloadDigestMismatch)
  - no key was inserted
```

### GWT-MRWE-2.3 — Metadata mutation at put

```
Given:
  - First put: record_a with metadata_a, succeeds
  - Second put: record_b with the same digest but different accepted_at_seq

When:
  - journal.put_compiled_ir(&record_b)

Then:
  - returns Err(JournalError::MetadataMutation { digest: <D> })
  - the stored record is unchanged
```

### GWT-MRWE-3.1 — Full digest check fail-closed on missing config

```
Given:
  - level = DigestCheck::Full
  - config = None

When:
  - verify_digests(journal, run, …, level, None)

Then:
  - returns Err(RecoveryError::FullDigestCheckConfigMissing)
```

### GWT-MRWE-3.2 — Full digest check on absent action ABI slice

```
Given:
  - level = DigestCheck::Full
  - config = Some(DigestCheckConfig {
      action_abi_entries: None,
      policy_entries: Some(&[]),
    })

When:
  - verify_digests(…)

Then:
  - returns Err(RecoveryError::FullDigestCheckConfigMissing)
```

### GWT-MRWE-3.3 — Full digest check on action ABI mismatch

```
Given:
  - level = DigestCheck::Full
  - config = Some(DigestCheckConfig {
      action_abi_entries: Some(&[(ActionId(7), expected_e, found_f)]),
      policy_entries: Some(&[]),
    })
  - expected_e != found_f

When:
  - verify_digests(…)

Then:
  - returns Err(RecoveryError::ActionAbiMismatch {
      action_id: ActionId(7), expected: expected_e, found: found_f })
```

### GWT-MRWE-3.4 — WorkflowSourceOnly is strictly weaker than Full

```
Given:
  - level = DigestCheck::WorkflowSourceOnly
  - workflow source matches; compiled IR, action ABI, policy all mismatch

When:
  - verify_digests(…)

Then:
  - returns Ok(())  (the level does not require IR/ABI/policy checks)
```

### GWT-MRWE-5.1 — StepSucceeded vs SlotWritten kind collision

```
Given:
  - JournalEvent::StepSucceeded { run, seq, step, output }
  - JournalEvent::SlotWrittenEvent { run, seq, slot, value, extra, attempt }

When:
  - record_kind() is called on both

Then:
  - StepSucceeded.record_kind() == RecordKind::StepSucceeded (29)
  - SlotWrittenEvent.record_kind() == RecordKind::SlotWritten (12)
  - 29 != 12
```

---

## Acceptance

This contract is **model-complete** for the four named beads. It does NOT claim proof closure. `proof-planner` owns the obligation lattice; `proof-writer` owns the artifact authoring; `proof-reviewer` owns the disposition.

The artifact bundle under `bd/vb-mrwe.1/` (this directory) is the integration point for downstream agents.

## References

- `domain-model.md` — ubiquitous language and forbidden states.
- `type-contracts.md` — function signatures with source citations.
- `workflow-model.md` — state machines for decode/put/recover.
- `error-taxonomy.md` — exact variant names and drift notes.
- `boundary-map.md` — parser/core/admission/recovery layering.
- `hazard-analysis.md` — H-1 through H-15 with current defenses.
- `proof-seeds.jsonl` — `proof-seed/v1` rows for `proof-planner`.
- `traceability-matrix.jsonl` — bead ↔ requirement ↔ source-file matrix.

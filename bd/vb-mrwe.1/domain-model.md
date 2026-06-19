# Domain Model — Storage Envelope & Digest Verification Family

**Beads covered**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`
**Surface**: `crates/vb_storage` — codec, journal/admission, recovery/types/digest, recovery/recover, records/kinds
**Skill**: `rust-contract/v1`
**Status**: Grounded in the **current** implementation as of HEAD (`969d1219c`).

## Ubiquitous Language

| Term | Definition | Source-of-truth file |
|---|---|---|
| **Record envelope** | The 60-byte header (magic, schema version, kind, header_len, payload_len, sequence, BLAKE3 digest of payload, CRC32C of preceding bytes) followed by the declared payload bytes. | `codec/header.rs`, `constants.rs` |
| **Magic word** | 4-byte magic that gates a record-kind family (e.g. `MAGIC_JOURNAL_EVENT`, `MAGIC_WORKFLOW_SOURCE`, `MAGIC_COMPILED_ARTIFACT`, `MAGIC_RECOVERY_STAMP`). A record is rejected if its magic does not match the family implied by its kind. | `codec/validation.rs`, `constants.rs` |
| **Trailing bytes** | Bytes in the input slice that lie strictly after `RECORD_HEADER_BYTES + header.payload_len`. These are an evidence-tampering signal and MUST be rejected. | `codec/payload.rs::reject_trailing_bytes` |
| **Payload digest** | `blake3::hash(payload)` (32 bytes) stored in the envelope header and verified at decode time AND at put time. | `codec/header.rs::build_record_header`, `codec/payload.rs::verify_digest_match`, `journal/admission.rs::verify_content_digest` |
| **Workflow source record** | A `WorkflowSourceRecord { source: Vec<u8>, digest: WorkflowDigest }` keyed by digest. The put path MUST verify `blake3(source) == digest` before insert. | `journal/source.rs::put_workflow_source` |
| **Compiled IR record** | A `CompiledIrRecord` with an inner accepted artifact envelope (`ir`, `warnings`, `required_capabilities`, `verification`, `accepted_at_seq`). The put path MUST verify (a) `blake3(ir) == digest`, (b) the artifact envelope is structurally valid, and (c) any re-write of the same digest carries the same metadata hash. | `journal/source.rs::put_compiled_ir`, `journal/admission.rs::validate_compiled_ir_record` |
| **Metadata mutation attack** | A write that targets an existing compiled-IR digest with a different accepted envelope (e.g. swapped `accepted_at_seq`, different `required_capabilities`, different `verification` flags). Defended by `compute_artifact_metadata_hash` over the canonical envelope fields. | `journal/source.rs::validate_metadata_hash_is_consistent`, `error::JournalError::MetadataMutation { digest }` |
| **Recovery digest check level** | An enum `DigestCheck { WorkflowSourceOnly, WorkflowAndIr, Full }` that dictates which digest classes are verified at a recovery boundary. Strict monotonicity: rank(WorkflowSourceOnly)=1 < rank(WorkflowAndIr)=2 < rank(Full)=3. | `recovery/types/digest.rs` |
| **Action ABI digest entries** | A `&[(ActionId, WorkflowDigest, WorkflowDigest)]` slice of `(action_id, expected, found)` triples passed at `DigestCheck::Full`. First mismatch produces `RecoveryError::ActionAbiMismatch`. | `recovery/recover.rs::check_action_abi_digests`, `recovery/digest.rs::first_action_abi_mismatch` |
| **Policy digest entries** | A `&[(StepIdx, WorkflowDigest, WorkflowDigest)]` slice of `(step, expected, found)` triples passed at `DigestCheck::Full`. First mismatch produces `RecoveryError::PolicyDigestMismatch`. | `recovery/recover.rs::check_policy_digests`, `recovery/digest.rs::first_policy_mismatch` |
| **StepSucceeded event** | A `JournalEvent::StepSucceeded { run, seq, step, output }` variant emitted when a workflow step completes successfully, distinct from `SlotWritten` (which records a side-effect slot write). | `events.rs`, `records/kinds.rs::StepSucceeded = 29` |

## Aggregates

The storage layer has **one** aggregate: the durable journal keyed by `(magic, record_kind)` and (for source/IR records) keyed by content digest. All invariant enforcement happens at the **put boundary** and the **decode boundary**. There is no in-place mutation of records once written; the only "update" surface for compiled IR is the same-digest metadata-equality check that rejects divergent re-writes.

## Policies

| Policy ID | Statement | Enforcement site |
|---|---|---|
| `P-ENVELOPE-STRICT-LENGTH` | `decode_record` MUST reject inputs where `bytes.len() != RECORD_HEADER_BYTES + header.payload_len` with `JournalError::UnexpectedTrailingBytes { declared_end, actual_len }`. | `codec/payload.rs:93`, `codec/envelope.rs:47` |
| `P-ENVELOPE-DIGEST` | At decode, `verify_digest_match(payload, header.payload_digest)` MUST be called and MUST return `JournalError::PayloadDigestMismatch` on mismatch. At put, `verify_content_digest(content, expected)` MUST be called for workflow source records and compiled IR records. | `codec/payload.rs:92`, `journal/admission.rs:5` |
| `P-COMPILED-IR-METADATA-IMMUTABLE` | Re-writing a compiled IR record under the same digest with a different metadata hash is rejected with `JournalError::MetadataMutation { digest }`. | `journal/source.rs:94-118` |
| `P-DIGEST-CHECK-STRICT-ORDERING` | `DigestCheck::hierarchy_rank()` is strictly monotonic. `checks_workflow_source() / checks_compiled_ir() / checks_full()` are derived from rank, not from independent flags. | `recovery/types/digest.rs:21-45` |
| `P-FULL-DIGEST-FAIL-CLOSED` | `verify_digests(_, _, _, _, DigestCheck::Full, None)` MUST return `Err(FullDigestCheckConfigMissing)`. Same-digest with `action_abi_entries: None` or `policy_entries: None` MUST also return `Err(FullDigestCheckConfigMissing)`. | `recovery/recover.rs:132-155` |
| `P-KIND-FAMILY-INVARIANT` | Every `(magic, kind)` pair accepted by `validate_kind_family` has a single, immutable home. Recovery stamps (`MAGIC_RECOVERY_STAMP`, kind 7) are NOT accepted by the journal-event decoder; workflow sources and compiled IR are NOT accepted by the recovery-stamp decoder. | `codec/validation.rs::validate_kind_family`, `records/kinds.rs` |
| `P-KIND-ID-STABLE` | `RecordKind::id()` MUST be stable across versions; reassigning wire IDs is a breaking change. | `records/kinds.rs:79-109` |

## Forbidden States

The following states MUST be unrepresentable after a successful `put` or `decode`:

1. A record whose header-declared payload length does not match the actual payload bytes supplied.
2. A record whose header-declared BLAKE3 digest does not match the actual payload bytes.
3. A compiled IR record whose metadata hash disagrees with the existing record at the same digest.
4. A journal event whose `record_kind()` would collide with another event variant at decode time.
5. A `DigestCheck::Full` verification that accepted without action ABI and policy entries being provided and matching.

The following states MUST produce a typed error rather than panic or silently corrupt:

1. `JournalError::UnexpectedTrailingBytes` for any slice with extra bytes past the declared payload.
2. `JournalError::PayloadDigestMismatch` for any envelope whose digest disagrees with its payload.
3. `JournalError::MetadataMutation` for any compiled-IR re-write that mutates metadata at the same digest.
4. `RecoveryError::*DigestMismatch` family for any recovery-boundary digest comparison that fails.
5. `RecoveryError::FullDigestCheckConfigMissing` for any Full digest check with absent or partial config.

## Open Domain Questions

1. **User-prompt variant drift** — the task brief mentions `JournalError::TrailingBytes { found, expected }` and `JournalError::DigestMismatch { kind: WorkflowSource | CompiledIr }`. Neither variant exists. The codebase uses `UnexpectedTrailingBytes { declared_end, actual_len }` and `PayloadDigestMismatch` (envelope level) plus `RecoveryError::WorkflowSourceDigestMismatch` / `CompiledIrDigestMismatch` (recovery boundary) plus `MetadataMutation` (same-digest divergence). The contract adopts the **existing** names. Any rename requires a separate bead.
2. **vb-mrwe.5 — what does StepSucceeded mean for `engine.step_state`?** The contract treats it as a durable event type only. The runtime engine's per-frame step state is a separate domain (vb-y4pa contract family) and is **not** in scope for this contract.

# Domain Model Review — vb-qi37.1.5

## Domain Model Summary

The recovery system uses a content-addressable `WorkflowDigest = [u8; 32]` to detect workflow artifact corruption during journal replay.

### Core Domain Types

| Type | File | Role |
|---|---|---|
| `WorkflowDigest` | `vb_core::ids` | 32-byte content identity of compiled workflow |
| `RecoveryError` | `vb_storage::recovery::types` | Typed error taxonomy with 10 variants |
| `DigestCheck` | `vb_storage::recovery::types` | Three-level verification policy |
| `UnsupportedRecoveryState` | `vb_storage::recovery::types` | Four-bit flag set for partial state support |
| `RecoveryFrameSeed` | `vb_storage::recovery::types` | Live-frame reconstruction seed |
| `ActionReplayTracker` | `vb_storage::recovery::types` | Non-idempotent action blocking set |

### Digest Mismatch Detection Flow

```
Journal RunAccepted.workflow (found)
        vs.
expected WorkflowDigest (reference)
        ↓
check_workflow_source_digest(journal, run, expected)
        ↓
Ok(()) — digests match
Err(WorkflowSourceDigestMismatch { expected, found }) — mismatch detected
Err(NoRecoveryData { run }) — no RunAccepted event
```

### Error Taxonomy Assessment

**Exhaustive**: All 10 `RecoveryError` variants cover distinct failure modes:
- 2 digest mismatches (workflow source, compiled IR)
- 2 deferred digest checks (action ABI, policy) — not yet instantiated
- 1 non-idempotent action blocking
- 1 replay divergence
- 1 no recovery data
- 1 corrupt snapshot
- 1 terminal state mismatch
- 1 frame dimension overflow

**Assessment**: The error taxonomy is well-designed. The deferred variants (`ActionAbiMismatch`, `PolicyDigestMismatch`) are intentionally future-proof placeholders — they are correctly noted as out of scope for this bead.

### Illegal State Analysis

**Representable illegal states**: None identified.

- `WorkflowDigest` cannot be constructed with fewer than 32 bytes — the type system enforces validity
- `EventSeq` is a monotonically increasing counter — cannot go backwards
- `RecoveryError` variants are exhaustive enums — Rust match is compile-time exhaustive
- `UnsupportedRecoveryState::union` is a pure monotonic flag union — no contradictory state possible

**Assessment**: Illegal states are unrepresentable. The domain model is sound.

### Key Verification Points

1. **Digest comparison is byte-exact** — no hash function involved, just `[u8; 32]` equality
2. **Priority order in `verify_digests`** — workflow source is checked before IR digest; first error wins
3. **`reject_workflow_digest_mismatch` behavior** — returns error when the first `RunAccepted` event has a mismatch (not when later events differ)
4. **`UnsupportedRecoveryState` flags** — set to `true` when the corresponding state cannot be reconstructed; used to alert the runtime boundary

### Open Questions (from codebase-map.md, answered)

1. **Action ABI digest verification is deferred** — Yes, out of scope for vb-qi37.1.5. The comment at `recover.rs:71-72` confirms this.
2. **Policy digest mismatch** — Never instantiated; out of scope for vb-qi37.1.5.
3. **Slot value drift** — `UnsupportedRecoveryState::slot_values_unsupported()` is the correct mechanism. The bead's corruption injection test `corrupt_slot_value_fails_with_slot_values_unsupported` will prove this path works.

### Scott Wlaschin DDD Assessment

**No type-driven design repair needed for this bead.** The domain model is already strict:
- `RecoveryError` is a sum type with named fields — no boolean blindness
- `UnsupportedRecoveryState` is a record of flags — no hidden state
- `DigestCheck` is a closed set of levels — no boolean explosion
- The `DigestMismatch` errors carry both `expected` and `found` values for diagnostic clarity

**Verdict**: Domain model is sound. Proceed to proof planning.

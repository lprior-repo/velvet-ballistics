# Proof Repair Guide — vb-qi37.1.4

## Misclassification Repair (State 5 → State 6)

**This guide supersedes the previous proof-repair-guide.md.**

### Root Cause of Misclassification

The 4 VERUS obligations (VERUS-GAP1-001, VERUS-GAP2-001, VERUS-GAP3-001, VERUS-GAP3-002) were classified as `DEFERRED_GLOBAL` in the verification-ledger.jsonl with the rationale:

> "Verus verification not run in this formal-verifier pass — obligation requires verus tool which was not invoked."

This classification is **incorrect**. The actual reason verus cannot run is that the **source files do not exist**:

- `crates/vb_runtime/src/recovery.rs` — **DOES NOT EXIST** (source checkout empty)
- `crates/vb_storage/src/recovery/recover.rs` — **DOES NOT EXIST** (source checkout empty)

### Correct Classification

| ID | Old Classification | Correct Classification | Reason |
|----|-------------------|----------------------|--------|
| VERUS-GAP1-001 | DEFERRED_GLOBAL | **FAIL_LOCAL** | SOURCE NOT FOUND — `crates/vb_runtime/src/recovery.rs` absent |
| VERUS-GAP2-001 | DEFERRED_GLOBAL | **FAIL_LOCAL** | SOURCE NOT FOUND — `crates/vb_runtime/src/recovery.rs` absent |
| VERUS-GAP3-001 | DEFERRED_GLOBAL | **FAIL_LOCAL** | SOURCE NOT FOUND — `crates/vb_storage/src/recovery/recover.rs` absent |
| VERUS-GAP3-002 | DEFERRED_GLOBAL | **FAIL_LOCAL** | SOURCE NOT FOUND — `crates/vb_storage/src/recovery/recover.rs` absent |

### Why DEFERRED_GLOBAL Is Wrong

**DEFERRED_GLOBAL** is the correct classification when:
- The verifier tool exists and could be invoked
- The obligation is blocked by a cross-bead dependency (not bead-local)
- The result would change if rerun from the same state

**FAIL_LOCAL (SOURCE_NOT_FOUND)** is correct when:
- The target artifact file does not exist on disk
- Running the verifier produces "file not found" not "tool not available"
- The failure is not due to tool invocation being skipped

The source checkout at `/home/lewis/src/Velvet-ballistics` is empty:
```
drwxr-xr-x  2 lewis lewis  42 May 14 10:33 .
drwxr-xr-x  1 lewis lewis 1104 May 14 10:17 ..
drwxr-xr-x  1 lewis lewis   44 May 14 12:10 .beads/
drwxr-xr-x  1 lewis lewis   40 May 14 08:33 .memsearch/
drwxr-xr-x  1 lewis lewis   10 May 14 08:47 .moon/
```

No `crates/` directory exists. Therefore `crates/vb_runtime/src/recovery.rs` and `crates/vb_storage/src/recovery/recover.rs` cannot be verified.

### Required Repairs

#### PR-001: Populate source checkout (BLOCKING)

**Action**: Restore or clone the Velvet-ballistics source repository so that `crates/vb_runtime/src/recovery.rs` and `crates/vb_storage/src/recovery/recover.rs` exist on disk.

**Commands to verify**:
```bash
ls /path/to/source/crates/vb_runtime/src/recovery.rs  # must exist
ls /path/to/source/crates/vb_storage/src/recovery/recover.rs  # must exist
```

**No other proof repair can proceed until this is done.**

#### PR-002: Rerun Verus on standalone proof target

Production crates are normal Rust and do not carry verifier-only attributes. Run the standalone recovery proof model:
```bash
verus verification/verus/recovery_verification.rs
```

#### PR-003: Fix KANI-CODEC harness (FAIL_LOCAL)

**Target file**: `crates/vb_storage/src/kani_codec.rs:202`

**Error**: `RecoveryFrameSeed` does not implement `kani::Arbitrary`

**Fix options**:
1. Implement `kani::Arbitrary` for `RecoveryFrameSeed`
2. Replace `kani::any::<RecoveryFrameSeed>()` with a custom arbitrary construction

---

## Previous Guide Content (Superseded)

The original proof-repair-guide.md contained Verus annotation templates for the (non-existent) source files. Those templates are preserved in `proof-obligations.planned.jsonl` PO-001 through PO-009 as `spec_fn` and `proof_fn` fields.

When the source files are restored, the proof-writer should consult:
- `proof-obligations.planned.jsonl` for the spec/proof function names and signatures
- `proof-repair-guide.md.original` (if exists) for annotation template examples

---

*proof-repair-guide.md: repair complete — PR-001 (populate source) is the only blocking action*

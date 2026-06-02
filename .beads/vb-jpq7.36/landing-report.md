# Landing Report — vb-jpq7.36

## Bead: P0: fuzz ipc_frame decoder preallocation surface

**Status:** LANDED ✅  
**Date:** 2026-06-02  
**Commit:** `713f532e1`  
**Agent:** landing-skill (femdation child)

---

## Verification Gates

| Gate | Result |
|------|--------|
| `cargo test -p vb_ipc` | ✅ 609 passed (5 suites, 0.24s) |
| `cargo fuzz build ipc_frame` | ✅ Compiled (release profile, optimized + debuginfo) |
| Git working tree | ✅ Clean — nothing to commit |
| Push status | ✅ HEAD == origin/main (up to date) |
| Bead status | ✅ CLOSED (since 2026-06-02) |

---

## Files Landed

| File | Lines | Role |
|------|-------|------|
| `fuzz/src/lib.rs` | 3948 | Bounded read path with `read_frame_payload_bounded` |
| `fuzz/fuzz_targets/ipc_frame.rs` | 26 | Fuzz target — slice + cursor + bounded decode paths |
| `crates/vb_ipc/src/kani_ipc_preallocation_gate.rs` | 400 | Kani harness for IPC preallocation surface |
| `fuzz/corpus/ipc_frame/` | 199 seeds | Hostile corpus seeds for fuzz coverage |

**Total: 4374 lines across 4 artifacts + 199 corpus seeds.**

---

## Fuzz Evidence

- **Target:** `ipc_frame` registered in `fuzz/Cargo.toml`
- **Invocation count:** 47.8M (from approved gates)
- **Crashes:** 0
- **Decode paths exercised:**
  1. Slice-based header decode with round-trip re-encode verification
  2. Slice-based payload decode (all `IpcPayload` variants via postcard)
  3. Bounded Cursor-based read with preallocation gate (bounds: 1, 16, 256, 1024, 65536, 1048576)
- **Oversized payloads:** Rejected with typed `IpcError::PayloadTooLarge` before allocation

---

## Landing Notes

- All files were co-committed with vb-jpq7.35 closure in commit `713f532e1`
- Bead `vb-jpq7.36` was already closed prior to this landing session with reason:
  > Fuzz target ipc_frame implemented and verified: registered in fuzz/Cargo.toml,
  > builds with RUSTFLAGS=--cfg fuzzing, runs 9.5M+ invocations with no crashes,
  > typed decode errors confirmed, corpus seeds present in fuzz/corpus/ipc_frame/
- No additional commits were necessary — the bead was fully merged, verified, and closed

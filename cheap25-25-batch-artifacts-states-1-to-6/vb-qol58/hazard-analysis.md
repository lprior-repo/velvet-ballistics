# Hazard Analysis — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)

This hazard analysis enumerates the **defensive-relevant** hazards at the
three production sites, classifies each by hazard kind, and identifies the
mitigation that the refactor applies. The refactor is a **preventive
tightening** — each hazard is one that the canonical verb either
eliminates or makes impossible.

## 1. Hazard Roster (per-site)

### Site A: `crates/vb_ipc/src/frame_types.rs:41` (`IpcFrameHeader::encode`)

| Hazard ID | Hazard | Class | Severity | Mitigation |
|---|---|---|---|---|
| `HAZ-A1-LEN-MISMATCH-SLICE` | `&mut bytes[..]` on a typed `[u8; N]` array is a redundant range expression; if the array length ever becomes a `&mut [u8]` variable (e.g., after a future refactor that parameterizes by `[u8; N]`), the `..` range becomes a runtime panic risk. | Rust-core invariant | Low (currently safe, latent risk) | Replace with `bytes.as_mut_slice()` (or `&mut bytes` auto-deref) which always returns a full-length `&mut [u8]` borrow. |
| `HAZ-A2-CURSOR-OVERFLOW` | If a future field is added to `IpcFrameHeader` and the `write_*` calls exceed `IPC_HEADER_LEN` bytes, the cursor would return `Err`. The current 7 calls total exactly 24 bytes; this is OK. | Bounded state | None (write window ≤ container length) | (No mitigation needed; the refactor preserves the cursor overflow check.) |
| `HAZ-A3-IPC-WIRE-INCOMPAT` | If `IPC_HEADER_LEN` is ever changed without updating the `write_*` call sequence, the encoded wire format becomes invalid. | Release/API | None (out of bead scope) | (No mitigation needed in this bead; the constant is unchanged.) |
| `HAZ-A4-LINT-REGRESSION` | A future nightly-Rust bump could trip `clippy::indexing_slicing` on `&mut bytes[..]`. | Tooling | Low | Replace with canonical verb; invariant preserved across nightly bumps. |

### Site B: `crates/workspace_tests/src/test_util/seed.rs:23` (`SeededBytes::<N>::new`)

| Hazard ID | Hazard | Class | Severity | Mitigation |
|---|---|---|---|---|
| `HAZ-B1-LEN-MISMATCH-ARRAY` | `&mut bytes[..]` on `[u8; N]`; same as HAZ-A1, redundant full-slice. | Rust-core invariant | Low (latent risk) | Replace with `bytes.as_mut_slice()`. |
| `HAZ-B2-N-EQUALS-ZERO` | If `N == 0`, `rng.fill(&mut [u8; 0])` is a no-op but the canonical verb also handles it cleanly. | Bounded state | None (guarded by `if N == 0 { return None }`) | (Existing guard preserved.) |
| `HAZ-B3-NON-DETERMINISTIC-RNG` | If a non-deterministic RNG were ever substituted (e.g., `thread_rng`), the fixture would lose reproducibility. | Determinism | None (RNG is fixed to `StdRng::seed_from_u64`) | (No mitigation needed; RNG constructor is unchanged.) |
| `HAZ-B4-LINT-REGRESSION` | Same as HAZ-A4. | Tooling | Low | Same mitigation. |

### Site C: `crates/workspace_tests/src/test_util/fixture.rs:58` (`FixtureBuilder::build_bytes`)

| Hazard ID | Hazard | Class | Severity | Mitigation |
|---|---|---|---|---|
| `HAZ-C1-LEN-MISMATCH-VEC` | `&mut vec[..]` on `Vec<u8>`; redundant full-slice, currently `clippy`-clean but could trip on a future nightly bump. | Rust-core invariant | Low | Replace with `vec.as_mut_slice()`. |
| `HAZ-C2-CAPACITY-VS-LEN` | `vec![0u8; cap]` sets length = capacity = `cap`. `rng.fill(&mut vec[..])` writes into all `cap` bytes. No mismatch. | Bounded state | None | (Existing invariant preserved; the refactor changes only the borrow expression.) |
| `HAZ-C3-OVER-MAX-CAPACITY` | If a future caller bypasses `FixtureCapacity::new`, the vec could exceed `MAX_CAPACITY = 1 MiB`. | Bounded state | None (constructor is the only entry point) | (Out of bead scope.) |
| `HAZ-C4-LINT-REGRESSION` | Same as HAZ-A4. | Tooling | Low | Same mitigation. |

## 2. Hazard Class Summary

### 2.1 Temporal hazards (none)

None of the three sites has a temporal component. There are no retries,
no timeouts, no scheduling, no async, no ordering dependencies. The
hazard class is **not applicable**.

### 2.2 Concurrency hazards (none)

None of the three sites is concurrent. There are no shared references,
no atomics, no locks, no channels, no tasks. The hazard class is **not
applicable**.

### 2.3 Unsafe / provenance hazards (none)

All three sites are `#![forbid(unsafe_code)]` (the `vb_ipc` crate at
lib.rs). The refactor introduces **zero** `unsafe` blocks. The hazard
class is **not applicable**.

### 2.4 Parser / codec hazards (none new)

`encode` writes a fixed-layout header; `decode` parses the same. The
refactor only changes the writer-target borrow expression. No new
parsing logic, no new validation, no new error paths. The hazard class
is **not applicable** for this bead.

### 2.5 Hostile-input hazards (none)

None of the three sites takes user-controlled input directly. `encode`
takes the typed `IpcFrameHeader` fields (already validated upstream);
`SeededBytes::new` takes a `u64` seed; `FixtureBuilder::build_bytes`
takes a `u64` seed and a capacity-checked `FixtureBuilder`. The hazard
class is **not applicable** for this bead.

### 2.6 Bounded-state hazards (preserved)

- `encode`: write window (24 bytes) equals `IPC_HEADER_LEN`. Preserved.
- `seed.rs`: write window (`N`) equals array length. Preserved.
- `fixture.rs`: write window (`cap`) equals vec length. Preserved.

### 2.7 Rust-core invariant hazards (mitigated)

- `HAZ-A1`, `HAZ-B1`, `HAZ-C1` are all the same class: redundant
  full-slice on a typed container. The refactor eliminates this class
  entirely at the three production sites.

### 2.8 Performance hazards (none new)

The canonical verb `as_mut_slice()` compiles to the same machine code as
`&mut bytes[..]` (both produce a `&mut [u8]` of the same length). No
performance change.

### 2.9 Release / API hazards (none new)

The public API of `IpcFrameHeader::encode`, `SeededBytes::new`, and
`FixtureBuilder::build_bytes` is **unchanged**. The refactor is an
**internal** canonicalization; no semantic-version bump is required.

## 3. Cross-Site Hazard Inventory

| Class | Sites | Mitigation |
|---|---|---|
| **Lint-regression on full-slice range expression** | A, B, C | Replace `&mut X[..]` with `X.as_mut_slice()` (or `&mut X`). |
| **Latent length-mismatch on redundant range** | A, B, C | Same replacement; type-system length evidence replaces runtime range evidence. |
| **Cross-nightly-Rust divergence** | A, B, C | Same replacement; canonical verb is stable across nightly versions. |

## 4. Mitigations the Refactor Does NOT Cover (explicit non-mitigations)

| Site-not-covered | Hazard | Why out of scope |
|---|---|---|
| `crates/vb_ipc/src/tests.rs:1529-1852` (test-side) | 17 slice patterns + 1 literal index | Per `delivery-scope.jsonl` row 14, default-scope is production-only; test-side cleanup is filed for follow-up beads. |
| `crates/vb_ipc/src/client/tests.rs:272, 410-627` (test-side) | 6 `&buf[..N].try_into().unwrap()` sites | Same as above. |
| `crates/vb_ipc/src/server/impl_tests.rs:419-715` (test-side) | 16 `bad_header[..N].copy_from_slice(…)` sites | Same as above. |
| `crates/vb_ipc/src/frame/tests.rs:119-1051` (test-side) | 26 slice patterns | Same as above. |
| `crates/vb_ipc/src/frame_types/tests.rs:18-105` (test-side) | 5 slice patterns | Same as above. |
| `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs:719-1181` | 3 `json.get("deltas").unwrap()` | Follow-on to `vb-hwkqa` precedent; out of default scope. |
| `crates/vb_cli/src/commands_diff/tests.rs:391-469` | 6 `outcomes.get(&n).unwrap()` | Same. |
| `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:86-786` | 14 `key[..N].try_into().unwrap()` | Same. |
| All `crates/*/src/kani_*.rs` (`#[cfg(kani)]` modules) | n.a. | Kani harnesses use `kani::any()`-generated lengths and are explicitly out of lint-src scope per delivery-scope row 17. |

## 5. Refactor Outcomes vs Hazards

| Hazard | Outcome after refactor |
|---|---|
| `HAZ-A1`, `HAZ-B1`, `HAZ-C1` (full-slice redundancy) | **Eliminated** — canonical verb replaces range expression at all 3 sites. |
| `HAZ-A2` (cursor overflow) | **Preserved** — write sequence unchanged; cursor's existing `Err` path remains. |
| `HAZ-B2` (`N == 0`) | **Preserved** — `if N == 0 { return None }` short-circuit unchanged. |
| `HAZ-C2` (capacity vs length) | **Preserved** — `vec![0u8; cap]` initialization unchanged. |
| `HAZ-A4`, `HAZ-B4`, `HAZ-C4` (lint regression) | **Mitigated** — canonical verb is lint-stable. |

## 6. Proof-Seed Lane Classification (preview for `proof-seeds.jsonl`)

Each site has a corresponding proof seed with the following lane hint
(this is a hint only; proof-planner owns final lane decisions):

| Site | Lane hint | Reason |
|---|---|---|
| A (`frame_types.rs:41`) | `rust-local` | Canonicalization only; pre-existing kani harnesses at `kani_ipc_header.rs` cover the encode/decode behavior. |
| B (`seed.rs:23`) | `rust-local` | Determinism is already covered by the existing `seeded_bytes_determinism` test at `seed.rs:33-37`. |
| C (`fixture.rs:58`) | `rust-local` | Determinism is implicit (RNG-driven); existing `FixtureBuilder` tests cover capacity boundaries. |

No `verus` / `kani` / `flux` / `loom` / `fuzz` / `tla` is needed because:

- No new behavior is introduced.
- The pre-existing kani harnesses in `vb_ipc` already cover the encode /
  decode state machine.
- The pre-existing unit tests in `seed.rs` and `fixture.rs` already cover
  the determinism / boundary cases.

## 7. Anti-Hallucination Markers

- All three sites are read live from this isolated workspace.
- All `IpcError::HeaderEncodeFailed` emit sites correspond exactly to
  the 7 `cursor.write_u*` lines in `frame_types.rs`.
- The `if N == 0 { return None }` guard at `seed.rs:18-20` is preserved
  by the refactor.
- No new hazards are invented; this analysis only documents what is
  already there plus the canonicalization-class hazard.
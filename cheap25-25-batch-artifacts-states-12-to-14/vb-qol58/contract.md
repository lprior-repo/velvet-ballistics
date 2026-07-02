# Contract — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)
> Owner-of-record: `rust-contract` (this file); downstream: `holzman-rust`, `proof-planner`, `proof-writer`, `formal-verifier`

This is the **top-level contract** for the bead. It aggregates the
domain, type, workflow, error, boundary, and hazard analyses into a
single decision-grade document that downstream agents (proof-planner,
proof-writer, holzman-rust, formal-verifier) can act on without
re-reading the supporting analyses.

## 1. Bead Premise (verbatim from delivery-scope.jsonl)

> Multiple source files contain unchecked slicing/indexing that violates
> lint-src. The pattern is `&mut bytes[..]`, `&buf[..N].try_into().unwrap()`,
> or hardcoded partial slices of fixed-size arrays. Refactor to canonical
> typed-byte-container accessors (`.first_chunk::<N>()`, `as_mut_slice()`,
> `try_into().unwrap()` → `expect(...)` or `.first_chunk()`) so the lint-src
> gate remains green and future refactors cannot panic at runtime.

**Verification status from exploration**: the lint-src gate is **already
green** at the 3 production sites (delivery-scope.jsonl rows 1, 2, 3).
The refactor is **preventive canonicalization**, not a bug fix that
unblocks a failing gate. All 3 sites are currently
`lint_class_now: "clippy::indexing_slicing (NO — ... lint-clean)"`.

## 2. Contract Statement

The bead commits to the following four contracts:

### C-1. `CONTRACT-LINT-CURSOR-MUT-ARRAY`

**For** `fn IpcFrameHeader::encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError>` at `crates/vb_ipc/src/frame_types.rs:39`.

**MUST** borrow the local `bytes: [u8; IPC_HEADER_LEN]` mutably via
`bytes.as_mut_slice()` (canonical verb) or `&mut bytes` (auto-deref)
when constructing the `Cursor` writer.

**MUST NOT** use `&mut bytes[..]` (redundant full-slice).

**MUST** preserve the 7-call sequence of `cursor.write_uXX<LittleEndian>` exactly.

**MUST** preserve the `Err(IpcError::HeaderEncodeFailed)` mapping on every cursor write failure.

### C-2. `CONTRACT-LINT-RNG-FILL-ARRAY`

**For** `fn SeededBytes::<N>::new(seed: u64) -> Option<Self>` at `crates/workspace_tests/src/test_util/seed.rs:17`.

**MUST** borrow the local `bytes: [u8; N]` mutably via `bytes.as_mut_slice()`
(canonical verb) or `&mut bytes` (auto-deref) when calling `rng.fill(...)`.

**MUST NOT** use `&mut bytes[..]`.

**MUST** preserve the `if N == 0 { return None }` guard exactly.

**MUST** preserve the deterministic `StdRng::seed_from_u64(seed)` constructor.

### C-3. `CONTRACT-LINT-RNG-FILL-VEC`

**For** `fn FixtureBuilder::build_bytes(self, seed: u64) -> Vec<u8>` at `crates/workspace_tests/src/test_util/fixture.rs:52`.

**MUST** borrow the local `vec: Vec<u8>` mutably via `vec.as_mut_slice()`
when calling `rng.fill(...)`.

**MUST NOT** use `&mut vec[..]`.

**MUST** preserve the `vec![0u8; self.capacity.value]` initialization.

**MUST** preserve the deterministic `StdRng::seed_from_u64(seed)` constructor.

### C-4. `CONTRACT-LINT-GATE-PRESERVED`

**For** the workspace `lint-src` moon task at `.moon/tasks/all.yml:46-53`.

**MUST** continue to deny `clippy::indexing_slicing`,
`clippy::get_unwrap`, `clippy::unwrap_used`, `clippy::string_slice`,
`clippy::arithmetic_side_effects`, `clippy::as_conversions`, etc. with
the existing `-D` flags.

**MUST NOT** loosen or weaken any deny flag.

**MUST** return `EXIT=0` after the refactor is applied.

## 3. Behavior Change Statement

| Function | Behavior change? |
|---|---|
| `IpcFrameHeader::encode` | **None** — return type unchanged, error variant unchanged, byte layout unchanged, write sequence unchanged. |
| `SeededBytes::<N>::new` | **None** — return type unchanged, edge-case `None` preserved, RNG seed and draw order unchanged. |
| `FixtureBuilder::build_bytes` | **None** — return type unchanged, RNG seed and draw order unchanged, vec length unchanged. |
| Workspace lint gate | **None** — flag set unchanged, deny-list unchanged. |

## 4. Anti-Regression Invariants

| ID | Invariant | Holds because |
|---|---|---|
| `INV-CB-1` | `IpcFrameHeader::encode` returns the same 24 bytes for the same `self`. | The 7 `write_uXX<LittleEndian>` calls happen in the same order on the same buffer length. |
| `INV-CB-2` | `IpcFrameHeader::encode` returns `Err(IpcError::HeaderEncodeFailed)` for the same trigger conditions. | The cursor's `write_*` error path is unchanged. |
| `INV-CB-3` | `SeededBytes::<0>::new(_)` returns `None`. | The `if N == 0` guard runs before any RNG work. |
| `INV-CB-4` | `SeededBytes::<N>::new(seed)` for `N > 0` returns the same byte sequence as before. | `StdRng::seed_from_u64` is deterministic; the fill window is identical. |
| `INV-CB-5` | `FixtureBuilder::build_bytes(self, seed)` returns `Vec<u8>` of length `self.capacity.value`. | `vec![0u8; cap]` initializes exactly `cap` elements; fill writes exactly `cap` bytes. |
| `INV-CB-6` | `FixtureBuilder::build_bytes(self, seed)` returns the same byte sequence as before for the same `seed`. | RNG is deterministic; fill window is identical. |
| `INV-CB-7` | The `lint-src` moon task continues to deny the same lint flags and exit 0. | The gate's deny list is not modified. |

## 5. Verification Approach

| Lane | Verifier | Why |
|---|---|---|
| Rust-local | `cargo clippy --workspace --lib --bins --examples --all-features -- -D clippy::indexing_slicing -D clippy::get_unwrap -D clippy::unwrap_used` | The lint-src gate. Currently EXIT=0; must remain EXIT=0. |
| Rust-local | `cargo check --workspace --all-targets --all-features` | Confirms all crates compile with the new accessor verb. |
| Rust-local | `cargo test -p vb_ipc` | Confirms the existing header encode/decode round-trip tests still pass. |
| Rust-local | `cargo test -p velvet-ballistics-workspace-tests` | Confirms the determinism and capacity-boundary tests in `seed.rs` and `fixture.rs` still pass. |
| Moon v2 | `moon run :lint-src` | The canonical gate per `.moon/tasks/all.yml:46-53`. |

No `verus` / `kani` / `flux` / `loom` / `fuzz` is needed because:
- The behavior is unchanged.
- Pre-existing kani harnesses in `vb_ipc` already cover the IPC encode /
  decode state machine (`kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`).
- Pre-existing unit tests in `seed.rs` (lines 33-50) cover determinism.
- Pre-existing unit tests in `fixture.rs` (lines 67-90) cover capacity boundaries.

## 6. Failure Conditions (refactor is REJECTED if any of these occur)

| ID | Condition |
|---|---|
| `FAIL-LINT-1` | `cargo clippy --workspace --lib --bins --examples --all-features` returns non-zero exit. |
| `FAIL-LINT-2` | Any new `clippy::indexing_slicing`, `clippy::get_unwrap`, or `clippy::unwrap_used` warning is introduced. |
| `FAIL-LINT-3` | The lint-src deny list is weakened (any `-D` flag removed). |
| `FAIL-TEST-1` | `cargo test -p vb_ipc` fails (existing round-trip tests). |
| `FAIL-TEST-2` | `cargo test -p velvet-ballistics-workspace-tests` fails (existing determinism tests). |
| `FAIL-BEHAVIOR-1` | `IpcFrameHeader::encode` returns different bytes for the same input. |
| `FAIL-BEHAVIOR-2` | `SeededBytes::<N>::new(seed)` returns different bytes for the same seed. |
| `FAIL-BEHAVIOR-3` | `FixtureBuilder::build_bytes(self, seed)` returns different bytes for the same seed. |
| `FAIL-BEHAVIOR-4` | `SeededBytes::<0>::new(_)` returns `Some(_)`. |
| `FAIL-BEHAVIOR-5` | `IpcFrameHeader::encode` returns a different `Err` variant. |

## 7. Out of Scope (explicit non-goals)

The following are explicitly NOT in this bead's contract; they are filed
for follow-up beads per `delivery-scope.jsonl` row 14:

1. Test-side literal-range slicing in `crates/vb_ipc/src/{tests,frame/tests,frame_types/tests,client/tests,server/impl_tests}.rs`.
2. `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs` `json.get(...).unwrap()` sites (3).
3. `crates/vb_cli/src/commands_diff/tests.rs` `outcomes.get(&n).unwrap()` sites (6).
4. `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` `key[..N].try_into().unwrap()` sites (14).
5. Any `kani_*.rs` harness (`#[cfg(kani)]` modules).
6. Adding a `lint-tests` moon task analogous to `lint-src` but scanning `--tests`.
7. Helper extraction (`pub(crate) fn header_bytes(...)`) for cross-module reuse in test-side cleanups.

## 8. Open Questions for Downstream Owners

1. **Should the auto-deref form (`&mut bytes`) or the canonical verb
   form (`bytes.as_mut_slice()`) be preferred?** Either is acceptable;
   `delivery-scope.jsonl` row 1 explicitly recommends `as_mut_slice()` for
   symmetry with `decode`'s `bytes.as_slice()` at `frame_types.rs:71`.
   Default: use `as_mut_slice()` for all three sites.
2. **Should `seed.rs` and `fixture.rs` use the same form?** Yes — use
   `as_mut_slice()` for consistency.
3. **Should the refactor also touch test-side sites?** No — explicitly
   out of default scope per row 14; file follow-up beads.

## 9. Risk Tags (per delivery-scope.jsonl)

- `lint-hygiene` (all 3 sites)
- `lint-hygiene, test-cleanup` (test-side, out of scope)
- `low-scope, low-risk` (overall bead posture)
- `first-principles-do-not-weaken` (GOD RULE on the gate)

## 10. Anti-Hallucination Markers

- All three source-line citations are read live in this isolated
  workspace.
- All 7 `IpcError::HeaderEncodeFailed` emit sites correspond exactly to
  the 7 `cursor.write_uXX<LittleEndian>` lines.
- The `if N == 0` guard at `seed.rs:18-20` is preserved verbatim.
- The `FixtureCapacity::MAX_CAPACITY` bound at `fixture.rs:11` is
  preserved verbatim.
- No new error variants, no new functions, no new types are introduced.
- The contract is a **canonicalization** contract: it forbids a
  redundant form, it does not invent new domain surface.
# State 8 Repair Report — vb-qi37.16.4

**bead_id:** vb-qi37.16.4
**phase:** state-8 repair
**date:** 2026-05-11
**STATUS:** BLOCKED (pre-existing global)

---

## Failure Classification

| Failure | Category | Classification | Fix Applied |
|---------|----------|----------------|-------------|
| FORMAT drift (97 files) | FORMAT | BLOCK_LOCAL | `cargo fmt` — all 97 files fixed |
| `ResourceContract` missing `allows_secret_results` (11 sites) | COMPILE | BLOCK_LOCAL | `..ResourceContract::DEFAULT` applied to all 11 literals |
| `vb_ipc/handlers.rs:243` `encoded` not in scope | COMPILE | DEFERRED_GLOBAL | NOT FIXED — pre-existing, outside scope |

---

## Commands Run

### 1. `cargo fmt --check` (post-fix)

```bash
$ rtk cargo fmt -- --check
(no output — clean)
```

**STATUS: PASS**

### 2. `rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"`

```bash
$ rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"
cargo test: 12 passed, 1337 filtered out (1 suite, 0.00s)
```

**STATUS: PASS**

### 3. `rtk cargo test -p vb_runtime --lib`

```bash
$ rtk cargo test -p vb_runtime --lib
cargo test: 1349 passed (1 suite, 0.27s)
```

**STATUS: PASS**

### 4. `rtk cargo check -p vb_codegen -p vb_core --all-targets`

```bash
$ rtk cargo check -p vb_codegen -p vb_core --all-targets
cargo build: 0 errors, 1 warnings (28 crates)
```

**STATUS: PASS**

### 5. `rtk cargo check --workspace --all-targets`

```bash
$ rtk cargo check --workspace --all-targets
error[E0425]: cannot find value `encoded` in this scope
    --> crates/vb_ipc/src/server/handlers.rs:243:22
     |
243 |         encoded_len: encoded.len() as u32,
     |                      ^^^^^^^ not found in this scope
```

**STATUS: FAIL** — only remaining error

---

## Minimal Holzman Rust Fixes Applied

### FORMAT (BLOCK_LOCAL)

- Ran `rtk cargo fmt` to fix all 97 files with formatter drift
- No manual edits; formatter corrected all indentation, import ordering, and line-wrapping issues

### `allows_secret_results` Compile Fallout (BLOCK_LOCAL)

Files edited (11 `ResourceContract` literals → added `..ResourceContract::DEFAULT`):

| File | Lines |
|------|-------|
| `crates/vb_codegen/src/tests.rs` | 902, 1458, 2106, 4352, 10152, 10196 |
| `crates/vb_codegen/src/proptests.rs` | 296 |
| `crates/vb_core/src/budget/tests.rs` | 1210 |
| `crates/vb_core/src/engine/validate.rs` | 223 |
| `crates/vb_core/src/workflow/tests.rs` | 686, 4497 |

All 11 sites now use `..ResourceContract::DEFAULT` to fill the new `allows_secret_results: bool` field with its conservative default value `false`.

---

## Not Fixed — Pre-Existing Global

### `vb_ipc/src/server/handlers.rs:243` — `encoded` not in scope

- **Classification:** `DEFERRED_GLOBAL`
- **Evidence:** Regression-diff explicitly states: "already observed during State 7 smoke as outside immediate `vb_runtime` smoke scope, but it is a global gate failure and must be compared to baseline before landing."
- **Scope exclusion:** `vb_ipc` is NOT in `touched_crates` for vb-qi37.16.4
- **Fix required:** `encoded` variable is referenced but never defined in `handle_answer_ask`. Likely needs `payload.len()` captured before `decode_payload` consumes it, or computed from the encoded answer representation. This is a pre-existing bug from a prior bead and must be compared to baseline before landing.

---

## Residual Risk

1. **BLOCKED on vb_ipc compile error** — global gate cannot pass until `encoded` issue is resolved in its originating bead or as a separate global fix
2. All FORMAT and `allows_secret_results` fixes are bead-local scoped fallout; no broadened scope

---

## Classification Summary

| Gate | Result |
|------|--------|
| `cargo fmt --check` | PASS |
| `cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"` | PASS (12 passed) |
| `cargo test -p vb_runtime --lib` | PASS (1349 passed) |
| `cargo check -p vb_codegen -p vb_core --all-targets` | PASS (0 errors) |
| `cargo check --workspace` | BLOCKED (1 pre-existing error in vb_ipc) |

**Decision:** Do not advance to State 9. The only remaining gate failure is a pre-existing global issue in vb_ipc that is outside the delivery scope of vb-qi37.16.4. Route vb_ipc fix to its originating bead or a separate global repair.

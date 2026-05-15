# Verification Layers: vb-0253.2

## Boundary

- **Verus-owned kernel**: N/A — facade conversion is structural; no new pure Rust core invariants
- **TLA+ temporal model**: N/A — no temporal behavior changes
- **Theorem projection**: N/A — no theorem kernels required
- **Runtime shell**: `lib.rs` facade after conversion delegates to `bounded.rs`, `ingress.rs`, `error.rs`, `codec.rs`. All behavior unchanged.
- **External systems excluded from formal proof**: `crossbeam_channel` (trusted), `postcard` (trusted), `bytes` (trusted)

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Notes |
|---|---|---|---|
| INV-001 (one-canonical-MemoryIngress) | source-audit | static-scan | Verify only one definition in codebase |
| INV-002 (one-canonical-IngressFrame) | source-audit | static-scan | Verify only one definition |
| INV-003 (one-canonical-QueueCapacity) | source-audit | static-scan | Verify only one definition |
| INV-004 (one-canonical-MaxPayloadBytes) | source-audit | static-scan | Verify only one definition |
| INV-005 (one-canonical-BoundedPayload) | source-audit | static-scan | Verify only one definition |
| INV-006 (stable-re-exports) | compile-check | api-compat | Downstream crates compile; semver check |
| INV-007 (bounded-memory-invariant) | Fowler test | static-scan | Existing tests cover Full/Disconnected/Empty |
| INV-008 (payload-validation-invariant) | Fowler test | proptest | Existing tests + parse-don't-validate |
| INV-009 (one-canonical-IpcError) | source-audit | static-scan | Verify only one definition |
| INV-010 (no-unsafe) | static-scan | `#![forbid(unsafe_code)]` | clippy + source scan |
| INV-011 (no-concurrency-change) | compile-check | Fowler test | No new concurrency patterns |
| POST-001 (pub mod declarations) | compile-check | — | Must compile |
| POST-002 (re-exports) | compile-check | — | Must compile |
| POST-004 (duplicates removed) | source-audit | — | grep lib.rs for removed definitions |
| POST-007 (tests.rs imports updated) | compile-check | — | `cargo test -p vb_ipc` |
| PRE-001 (duplicate lines exist pre-condition) | source-audit | — | Confirmed in codebase-map.md |
| PRE-002 (module declarations absent pre-condition) | source-audit | — | Confirmed in codebase-map.md |

## Verus Scope

- **Rust target**: N/A
- **Spec/proof function**: N/A
- **Invariants**: N/A
- **Trusted boundary**: N/A
- **Shell exclusions**: N/A

## TLA+ Scope

- **Module/model path**: N/A
- **Variables**: N/A
- **Actions**: N/A
- **Safety invariants**: N/A
- **Temporal properties**: N/A
- **Fairness/deadlock stance**: N/A
- **Refinement boundary**: N/A
- **Evidence command**: N/A

## Theorem Scope

- **Theorem module**: N/A
- **Rust target**: N/A
- **Abstraction relation**: N/A
- **Shell exclusions**: N/A
- **Non-goals**: No theorem projection required for pure facade refactor

## Source-Audit Obligations (manual grep verification)

After implementation (State 10), verify:

```bash
# Exactly one MemoryIngress definition
rg 'struct MemoryIngress' crates/vb_ipc/src/ --stats  # expect only ingress.rs

# Exactly one IngressFrame definition
rg 'struct IngressFrame' crates/vb_ipc/src/ --stats  # expect only ingress.rs

# Exactly one QueueCapacity definition
rg 'struct QueueCapacity' crates/vb_ipc/src/ --stats  # expect only bounded.rs

# Exactly one MaxPayloadBytes definition
rg 'struct MaxPayloadBytes' crates/vb_ipc/src/ --stats  # expect only bounded.rs

# Exactly one BoundedPayload definition
rg 'struct BoundedPayload' crates/vb_ipc/src/ --stats  # expect only bounded.rs

# Exactly one IpcError definition
rg 'enum IpcError' crates/vb_ipc/src/ --stats  # expect only error.rs

# No map_try_send in lib.rs
rg 'fn map_try_send' crates/vb_ipc/src/lib.rs  # expect: no matches

# No u32_to_usize duplicate in lib.rs
rg 'fn u32_to_usize' crates/vb_ipc/src/lib.rs  # expect: no matches (only in error.rs)

# pub mod declarations exist
rg 'pub mod (bounded|ingress|error)' crates/vb_ipc/src/lib.rs  # expect: 3 matches
```

## Compile/Build Obligations

```bash
cargo build -p vb_ipc                    # facade compiles
cargo test -p vb_ipc                      # all tests pass
cargo build -p velvet_ballastics         # downstream crate compiles
cargo build -p workspace_tests           # downstream bench crate compiles
moon run :verify-standard                # full moon ci lane
```

## API-Compat Obligations

```bash
cargo semver-checks -p vb_ipc  # if semver checks exist for this crate
```

## Waivers

| Clause | Reason | Compensating Evidence |
|---|---|---|
| Any formal proof (Verus/Kani/Lean/TLA+) | Facade refactor is pure structural reorganization; all behavior unchanged and exercised by existing test suite | 60+ existing unit tests in tests.rs, client/tests.rs, server/impl_tests.rs, frame/tests.rs |
| TLA+ temporal model | No temporal behavior changes; MemoryIngress queue semantics unchanged | crossbeam_channel trusted runtime |
| Proptest/fuzz | Parse-don't-validate covered by existing boundary tests | `bounded_payload_rejects_oversized_with_exact_counts`, `adversarial_bounded_payload_rejects_exactly_one_over_max` |
| Miri/Loom/Kani | No unsafe code; no new concurrency patterns | `#![forbid(unsafe_code)]` on all vb_ipc files |

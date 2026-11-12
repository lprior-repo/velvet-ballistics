# vb-kyyf Proof Evidence - cfg-branch binding and ledger reconciliation attempt 4

## rerun_from

State 5 / cfg-branch-binding-and-ledger-reconciliation-after-state6-rejection sublane.

## Workspace

All commands run with workdir `/home/lewis/src/bd-vb-kyyf-bdd`.

## Ledger Reconciliation

### PO-008 / TLA-KYYF-001

- **Status change**: `blocked_tooling` → `planned`
- **Reason**: Contract-verification review identified stale discovery text claiming TLA spec does not exist. File `verification/tla/VbKyyfReplayDeterminism.tla` (26.5K) and `verification/tla/VbKyyfReplayDeterminism.cfg` (471B) both exist. Retained evidence from attempt 7 shows TLC completed with 0 errors: 42,907,696 states generated, 16,483,704 distinct, depth 9, 4min 37s with 32 workers.
- **Command updated**: Added `JAVA_TOOL_OPTIONS` and `-metadir` isolation for TLC temp/quota issues.

### PO-009 / VERUS-KYYF-001

- **Status change**: `blocked_tooling` → `planned`
- **Reason**: Contract-verification review identified stale discovery text claiming normalized observation kernel does not exist. File `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs` exists and contains the production kernel. The `#[cfg(verus_keep_ghost)]` branch (lines 8-326) carries Verus contracts; `verification/verus/vb_kyyf_normalization.rs` imports it via `#[path = ...]` (line 25-26) and verifies 42 obligations.
- **Artifact updated**: `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs`
- **Command updated**: `verus verification/verus/vb_kyyf_normalization.rs`

## cfg-branch Binding Analysis

### Finding: Production source has two hand-duplicated implementations

**Location**: `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs`

- `#[cfg(verus_keep_ghost)]` branch: lines 8-326 (Verus-annotated with `ensures` contracts)
- `#[cfg(not(verus_keep_ghost))] mod cargo_kernel`: lines 328-518 (safe Rust, no Verus annotations)
- `#[cfg(not(verus_keep_ghost))] pub use cargo_kernel::*`: line 517-518 (re-exports)

**Structural difference**: The two branches are not mechanically identical:
- Verus branch uses early `return` statements in `compare_replay` and `compare_generated_ir`
- Cargo branch uses if-else without early returns in the corresponding functions
- Both produce the same observable results, but the control flow is hand-duplicated

**Proof-review finding**: "the verified branch and cargo branch are still two hand-maintained implementations. Under GOD RULE 2, that is not yet strong enough to approve PO-009 as production-bound."

### Blocked production binding repair

The cfg-branch binding gap cannot be closed without production code changes:

1. **Remove the cfg split**: Collapse both branches into a single implementation that works with both Cargo and Verus
2. **Use Verus-compatible syntax**: Replace early returns with if-else blocks to maintain single control flow structure
3. **Or use a macro/conditional compilation**: Generate both versions from a single source

**This is a blocker**: As proof-writer, I cannot edit production code. This requires routing to an implementation repair sublane.

## Legacy Mirror Demotion

### Spec*/Exec* mirrors in Verus artifact

**Location**: `verification/verus/vb_kyyf_normalization.rs`

- `Spec*` types and proofs: lines 201-838 (excluding `Exec*` and `production_probe` sections)
- `Exec*` types and proofs: lines 289-469

**Demotion**: These are NOT production-binding evidence. They are standalone Verus spec/proof code that mirrors the production types but is not connected to the actual cargo-compiled implementation.

**Only production-binding obligations**:
- `production_probe::*` wrapper functions: lines 28-72
- `proof_prod_*` obligations: lines 74-195

**Demotion confirmed**: The Verus artifact contains a comment at line 840-856 explicitly listing trusted shell boundaries. The `Spec*` and `Exec*` layers do not appear in that trusted boundary list.

## Raw Command Evidence

### Verus verification

```bash
$ verus verification/verus/vb_kyyf_normalization.rs
verification results:: 42 verified, 0 errors
```

### Cargo tests

```bash
$ rtk cargo test -p vb_proof_kernels vb_kyyf_normalization --all-features
cargo test: 3 passed, 34 filtered out (1 suite, 0.00s)
```

### Strict clippy

```bash
$ rtk cargo clippy -p vb_proof_kernels --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo clippy: No issues found
```

### Trust shortcut scan

```bash
$ rtk grep -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom|admit|sorry|unimplemented|todo' verification/verus/vb_kyyf_normalization.rs crates/vb_proof_kernels/src/vb_kyyf_normalization.rs --glob '*.rs'
0 matches for 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom|admit|sorry|unimplemented|todo'
```

### TLA files exist

```bash
$ rtk ls verification/tla/VbKyyfReplayDeterminism.tla verification/tla/VbKyyfReplayDeterminism.cfg
verification/tla/VbKyyfReplayDeterminism.cfg  471B
verification/tla/VbKyyfReplayDeterminism.tla  26.5K
```

## Classification

| Obligation | Status | Classification | Notes |
|---|---|---|---|
| PO-008 / TLA-KYYF-001 | `planned` (was `blocked_tooling`) | `UNBLOCKED_LEDGER_RECONCILED` | TLA spec exists; TLC completed with 0 errors; ledger reconciled |
| PO-009 / VERUS-KYYF-001 | `planned` (was `blocked_tooling`) | `BLOCKED_CFG_BINDING` | 42 verified; production_probe bound; but cfg-branch hand-duplication is unresolved |

## Blocker Packet for Implementation Repair

**Type**: `cfg-branch-binding-requires-production-change`

**Artifact**: `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs`

**Required repair** (implementation owner, not proof-writer):
1. Remove the `#[cfg(verus_keep_ghost)]` / `#[cfg(not(verus_keep_ghost))]` split
2. Produce a single implementation that:
   - Works with Cargo (no Verus-specific syntax)
   - Works with Verus (Verus can parse and verify it)
   - Has the same control flow in both modes (no hand-duplicated early-return vs if-else)
3. The `verus!` block should contain the full implementation with `ensures` contracts
4. Cargo should compile the same source without Verus annotations

**Acceptable approaches**:
- Move the Verus-annotated implementation to a separate file included by both Cargo and Verus via `#[path]`
- Use a macro to generate both versions from a single declarative definition
- Restructure so Verus sees the annotated version and Cargo sees a version with contracts erased

**Proof artifact for PO-009 will require update after production repair**: Once production code is restructured, the Verus artifact's `#[path]` import will verify the same body that Cargo compiles.
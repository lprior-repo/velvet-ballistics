# Proof Evidence

- **Bead**: vb-b8i8f
- **State**: 5 (proof-writer)
- **Invocation**: vb-b8i8f-state5-proof-writer-attempt2

## Artifact Hashes (sha256)

| Path | SHA256 |
|------|--------|
| `crates/vb_storage/src/codec/validation.rs` | `b8b44ed0478766e2f07219017ef24d1082f9a2f0a27cdc454e34bd4b7925d9ef` |
| `verification/verus/cancel_kill_lattice.rs` | `bde67d827718093c0f5e4f607f93c406ad781aabfc17b880b100adbbcad4fbab` |
| `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs` | `da2deae89de369d624df7f8eefb05ece7dad1b6b3c84d44db9bb6aae690ff6e2` |
| `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` | `1c68aa20ced772d580152152c1b6c7a1eb8b3bbfdff2d83794e1cc7c6524a37a` |
| `crates/vb_storage/src/codec/flux_validation.rs` | `27cecfbefd3a0468796062b2d2d9c7445926e2b0e62af3f080706dbc8bcb6cce` |
| `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` | `42db2ccd084749a63a5536ae36ea102edbd5389520006c3485e3d3cddd8d176f` |
| `crates/vb_storage/src/proptest_storage.rs` | `b00111cd758f8dcdbaa682e48471ec01122f40d98808e17b2db319bd6969a292` |

## Assumptions

1. **TBR-001**: RunId is an opaque tracked identity type — IndexMap/IndexSet operations are defined for all u64 values
2. **TBR-002**: Shard construction from arbitrary initial state (for Kani bounded verification; BLOCKED)
3. **TBR-004**: Flux extern_spec for standard HashMap/IndexMap operations — membership predicates are pure
4. **TBR-006**: terminal_runs as tracked ghost state — IndexSet::insert is idempotent (monotonic)
5. **TBR-008**: MAGIC_* constants as trusted values — MAGIC_JOURNAL_EVENT=1447184965
6. **TBR-009**: RecordKind::id() as trusted mapping — RunKilled.id() == 28
7. **TBR-010**: Flux refinement for integer range predicates — 10..=28 is a valid Flux integer range
8. **TBR-011**: postcard round-trip stability — deterministic encode/decode for same input

## Model Bounds

- **Verus**: finite RunId domain abstracted as tracked ghost set; terminalization is a pure function over (run_state, command) pairs
- **Kani**: bounded to 3 concurrent runs; RunId values bounded to u16::MAX range; terminal_runs bounded to 8 entries; kani::unwind(3-4)
- **Flux**: refinement checks return discriminant only, not payload structure; refinement on integer range predicates
- **Proptest**: 1000 cases per property; RunId bounded to u64::MAX

## Trusted Boundaries

| ID | Boundary | Justification |
|----|----------|---------------|
| TBR-001 | RunId as opaque type | Used as IndexMap/IndexSet key; all u64 values are safe |
| TBR-008 | MAGIC_* constants | Verified from constants.rs via Kani PO-KANI-004 |
| TBR-009 | RecordKind::id() | Stable durable storage contract; verified by proptest PO-PROP-004 |
| TBR-FLUX-001 | #[flux_rs::trusted] handle_cancel model | Kani PO-KANI-001 verifies swap_remove semantics |
| TBR-FLUX-002 | #[flux_rs::trusted] handle_kill model | Kani PO-KANI-001 verifies swap_remove semantics |
| TBR-FLUX-003 | #[flux_rs::trusted] validate_kind_family wrapper | Kani PO-KANI-004 verifies exhaustive kind-space |
| TBR-FLUX-004 | #[flux_rs::trusted] is_known_record_kind wrapper | const fn; verified by Kani PO-KANI-004 |
| TBR-VERUS-001 | #[verifier::external_body] classify_run | Mathematical model; production verifies via integration tests |

## Raw Command Evidence

### Compile Check
```bash
$ cargo check -p vb_storage
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s

$ cargo check -p vb_runtime
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.65s
```

### Proptest Execution
```bash
$ cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props -- --nocapture
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

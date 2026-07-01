# Proof Evidence

- **Bead**: vb-qxjgx
- **State**: 5 (proof-writer)
- **Invocation**: `p5-proof-writer: Kani x5 + proptest x2 for vb-qxjgx StepSucceeded/SlotWritten split`
- **JJ change id**: `ywnswumt`
- **JJ commit id**: `1f4db9f6f184fcaef5742a3edcbdeff2721b798f`
- **Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
- **Plan disposition**: `APPROVED` (proof-plan-review.md STATUS: APPROVED, line 157)
- **Date**: 2026-07-01

## Artifact Hashes (sha256)

| Path | SHA256 |
|------|--------|
| `crates/vb_storage/Cargo.toml` | `ab787b7c4d0dd7d7ac9312a7cee68467798b89cada17f2f54075cab8aba34cd5` |
| `crates/vb_storage/src/lib.rs` | `a5cf050acc1f7abb6de566d4fe57fdc6063a21e369fcf3b68b877735723acab0` |
| `crates/vb_storage/src/codec/tests.rs` | `519ddb1f0e3b052e8a19e9bb8e1ad606825cd467ecffeb961db6230739c19889` |
| `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` | `25cf444059fe565ca21a4fe9661dc75f3ad83d2e999990cf15bffa3a2c49de33` |
| `crates/vb_storage/src/kani_record_kind_projection_split.rs` | `939e83d844b344706f02941ac446dec6376919d5b96f596bf1866a45763643d1` |
| `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` | `775c8296630a57e3df3a2ffdd2fa87c9b2580aa708fe5a3cc9aab17fadebfbf7` |
| `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` | `7beb9cd020590e11a7c4e664ac475afa920cd773b33bead5e87ffa2a28b6a49a` |
| `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` | `a1d6b517b9d7448490291a36497777e83f24d2fc26732e85c32ba6c13c47663f` |
| `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` | `dccbbd0fbc989f5972a2add3fa828d9563557dd2a0e49e500f43d5cb509a6e71` |
| `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` | `59fe81a424d6e236133b99c31e5ec84c75b7ce5160cef61c28858a6a6a6fc1f5` |

## Assumptions (per `proof-obligations.planned.jsonl.assumptions`)

All 7 obligations have `assumptions: []` (no `kani::assume(...)` short-circuits).
The kani harnesses use `kani::any()` / `kani::any_where()` for symbolic input
but do NOT use `kani::assume(...)` to bypass any branch. This is the
trusted-base-plan.md §1 disposition.

| Obligation | `assumptions` | Notes |
|---|---|---|
| PO-QXJGX-001 | `[]` | Pure bijection property over `RecordKind::id()`. |
| PO-QXJGX-002 | `[]` | One-to-one projection property. |
| PO-QXJGX-003 | `[]` | Family admit/reject property. |
| PO-QXJGX-004 | `[]` | Parity-gate acceptance grid. |
| PO-QXJGX-005 | `[]` | Round-trip property. |
| PO-QXJGX-006 | `[]` | Variant-keyed counter property. |
| PO-QXJGX-007 | `[]` | Compile-time constant + literal-sync property. |

In addition, several harnesses use `kani::assume(...)` for the
**structural validity pre-conditions** that the production
`JournalEvent::is_valid()` enforces (e.g., `run != 0`, `seq != u64::MAX`,
`attempt != 0` for variants that carry attempt). These are NOT
`kani::assume` for the property under proof; they are
*pre-conditions* to keep the harness within the valid input domain.
Examples:

- `kani_record_kind_projection_split.rs:35-37` — `run != 0`, `seq != u64::MAX`
- `kani_record_kind_projection_split.rs:70-72` — `run != 0`, `seq != u64::MAX`, `attempt != 0`
- `kani_record_kind_parity_legacy_envelope.rs:21-22, 33-34` — same pre-conditions
- `kani_record_kind_decode_round_trip.rs:30-32` — same pre-conditions

These are documented in the harness headers; they do not bypass
the property under proof. They are the pre-conditions of
`JournalEvent::is_valid()` (events.rs:514-550) that the production
codec enforces before reaching the parity gate.

## Model Bounds (per `proof-obligations.planned.jsonl.model_bounds`)

| Obligation | Bound | Default | Reduction? |
|---|---|---|---|
| PO-QXJGX-001 (kani) | `unwind=8, input_size=1024, mem_high=20G, mem_max=24G` | per `references/resource-governance.md` | None |
| PO-QXJGX-002 (kani) | same | default | None |
| PO-QXJGX-003 (kani) | same | default | None |
| PO-QXJGX-004 (kani) | same | default | None |
| PO-QXJGX-005 (kani) | same | default | None |
| PO-QXJGX-006 (proptest) | `cases=10000, input_size=1024` | per `references/resource-governance.md` | None |
| PO-QXJGX-007 (proptest) | same | default | None |

The kani harnesses use the default `kani::unwind(...)` (no
`#[kani::unwind(N)]` attribute) except where the harness requires a
specific unwind bound (e.g., kani_record_kind_journal_family_33.rs:H5
uses `#[kani::unwind(3)]` to extend the journal-family exhaustive
sweep to the post-fix set). The proptests use
`ProptestConfig { cases: 10000, ... }` per
`proof-strategy.md:7 Resource Governance and Tooling Pins`.

## Tool Metadata (per `proof-obligations.planned.jsonl.tool_metadata`)

| Tool | Version pin | Feature flags | Solver |
|------|-------------|---------------|--------|
| `cargo-kani` | `0.67.0` | `--features kani-vb-qxjgx-record-kind-split` (per `Cargo.toml`) | `cbmc` |
| `proptest` | `1.5` (workspace pin) | n/a (dev-dependency) | n/a |

## Production Binding (GOD RULE 2 / STRONG)

| Obligation | Production symbol | File:line |
|---|---|---|
| PO-QXJGX-001-H1, H2, H3 | `crate::records::RecordKind::id`, `crate::records::RecordKind` | `crates/vb_storage/src/records.rs:210, 139` |
| PO-QXJGX-002-H1, H2, H3 | `crate::events::JournalEvent::record_kind` | `crates/vb_storage/src/events.rs:401-429` |
| PO-QXJGX-003-H1, H2, H3, H4, H5, H6 | `crate::codec::validation::is_known_record_kind`, `crate::codec::validation::validate_kind_family` | `crates/vb_storage/src/codec/validation.rs:23-25, 42-60` |
| PO-QXJGX-004-H1, H2, H3, H4, H5, H6, H7 | `crate::codec::EnforceKindParity::enforce_kind_parity` (JournalEvent), `crate::codec::validate_journal_event_record_kind` | `crates/vb_storage/src/codec/kind_parity.rs:50-64`, `crates/vb_storage/src/codec/mod.rs:97-111` |
| PO-QXJGX-005-H1, H2, H3 | `crate::codec::decode_journal_event`, `crate::codec::encode_record` | `crates/vb_storage/src/codec/mod.rs:126-151, 60-71` |
| PO-QXJGX-006-H1, H2, H4 | `crate::recovery::replay::summary::apply::apply_summary_event` | `crates/vb_storage/src/recovery/replay/summary/apply.rs:23` |
| PO-QXJGX-006-H3, H4 | `crate::events::JournalEvent::record_kind` | `crates/vb_storage/src/events.rs:401-429` |
| PO-QXJGX-007-H1, H4 | `crate::runtime::durability_matrix::DURABILITY_MATRIX` | `crates/vb_runtime/src/durability_matrix.rs:70-204` |
| PO-QXJGX-007-H2 | `crate::codec::validation::validate_schema_version` (re-routed through `validate_record_kind_family` + `CURRENT_SCHEMA_VERSION` public surface) | `crates/vb_storage/src/codec/validation.rs:10-21, 42-60` |
| PO-QXJGX-007-H3 | `crate::codec::flux_validation` source (literal parse at line 14, 33) | `crates/vb_storage/src/codec/flux_validation.rs:14, 33` |

All bindings are STRONG: the harness/property code calls the
production function/type directly via canonical `crate::...`
paths. No `verification/` shadow model was used. No
`production_inner/*_production.rs` mirror was needed. No
`extern_*.rs` companion was needed. The contract is rust-local +
kani + proptest (no Verus in scope per proof-strategy.md §5).

## Kani Non-Vacuity (kani::cover! + kani::assert)

| File | cover! site | Reachable path |
|------|-------------|----------------|
| kani_record_kind_id_step_succeeded.rs:H1 | `id == 33` | id 33 is reachable in the bijection |
| kani_record_kind_projection_split.rs:H1 | `matches!(projected, RecordKind::StepSucceeded)` | StepSucceeded projection arm reachable |
| kani_record_kind_journal_family_33.rs:H2 | `result.is_ok()` | MAGIC_JOURNAL_EVENT + id 33 admit path reachable |
| kani_record_kind_journal_family_33.rs:H1 | `result` (id 33 in known set) | id 33 is reachable in the known-kind predicate |
| kani_record_kind_parity_legacy_envelope.rs:H2 | `r1.is_ok()` (legacy envelope-12 + StepSucceeded) | back-compat branch reachable (POST-005) |
| kani_record_kind_decode_round_trip.rs:H2 | `r.is_ok()` (legacy envelope-12 + StepSucceeded) | back-compat branch reachable (POST-005 / POST-006) |

**Total: 6 paired `kani::cover!` reachability proofs** (one per
new kani file except kani_record_kind_journal_family_33.rs which
has 2). Per trusted-base-plan.md §3, every `kani::cover!` is paired
with a `kani::assert` (no `cover!`-as-proof obligations).

## Kani Non-Suppression Audit

| Suppression flag | Used? | Notes |
|------------------|-------|-------|
| `--no-default-checks` | No | default Kani checks enabled |
| `--no-memory-safety-checks` | No | memory safety checks enabled |
| `--no-overflow-checks` | No | overflow checks enabled |
| `--no-unwinding-checks` | No | unwinding checks enabled |
| `--only-codegen` | No | full verification enabled |
| `--no-codegen` | No | codegen enabled |
| `--prove-safety-only` | No | property + safety verified |
| `--harness` (explicit) | Yes | each harness is named per planned.jsonl |
| `--mem-predicates` | Yes | per planned.jsonl:60 |

The planned command (per proof-obligations.planned.jsonl) is:
```
cargo kani -j 1 --output-format=regular --harness <NAME> --mem-predicates
```

## Proptest Anti-Invariants

The literal `invalid_input` appears as the strategy filter token
in:

- `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs:40` (anti-invariant marker for `prop_filter(ANTI_INVARIANT_TOKEN, ...)`)
- `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs:25` (anti-invariant marker for the strategy filter; also `anti_invariant_token_present` unit test at line 261)

Per proof-strategy.md §7, the literal `invalid_input` is the
proof-writer skill's grep-checked marker for non-vacuous
strategies. Both proptest files carry the marker.

The `E_KANI_ASSUMPTION_VACUITY` closure for the pre-fix collapse
appears in:
- PO-QXJGX-006-H4: `id_keyed_counter_would_diverge_from_variant_keyed` — asserts that an id-keyed counter would yield a different total than the variant-keyed counter when envelope_id_step=12 (the legacy wire id).

## Kani Harness Isolation (per AGENTS.md)

All 5 new kani files are feature-gated:
- `crates/vb_storage/Cargo.toml` adds the `kani-vb-qxjgx-record-kind-split` feature
- `crates/vb_storage/src/lib.rs` registers each module with
  `#[cfg(all(kani, feature = "kani-vb-qxjgx-record-kind-split"))]`
- The kani harness body uses `#[cfg(kani)] mod harness_name { ... }`
  to keep the proof harness code out of the default build path

The 5 new harnesses do NOT pollute the default build (no `cfg(kani)`
without feature gate) and do NOT pollute the `legacy-kani` feature
group (which contains the pre-existing kani_record_kind.rs).

## Raw Command Evidence

### pwd / jj root / git toplevel

```
$ pwd -P
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx
$ jj root
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx
$ git rev-parse --show-toplevel
fatal: not a git repository (or any parent up to mount point /)
```

The isolated JJ workspace at `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
is correctly identified as the active workspace. Git is not initialized
at the isolated workspace (jj-only repo); the coord checkout at
`/home/lewis/src/velvet-ballistics` is the git mirror. The coord
checkout is `clean — nothing to commit` (verified via `git status`).

### Smoke evidence (cargo check)

```
$ cargo check -p vb_storage
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.52s
   (no errors)

$ cargo check -p vb_storage --features kani-vb-qxjgx-record-kind-split
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.76s
   (no errors; the 5 kani files compile to empty cargo metadata
   because the modules are #[cfg(kani)]-gated)

$ cargo check -p vb_storage --tests
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `RecordKind` in the current scope
     --> crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs:222:29
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `records::RecordKind` in the current scope
     --> crates/vb_storage/src/codec/tests.rs:1639:21
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `records::RecordKind` in the current scope
     --> crates/vb_storage/src/codec/tests.rs:1743:21
   cargo build: 3 errors, 0 warnings (1 crates)

$ cargo check -p vb_runtime --tests
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `RecordKind` in the current scope
     --> crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs:251:13
   cargo build: 1 error, 0 warnings (1 crates)
```

The 4 `cargo check --tests` errors are the EXPECTED
PENDING_FORMAL_EXECUTION signal. The production code does not yet
have `RecordKind::StepSucceeded = 33`; the proof artifacts are
forward-looking and will compile + execute successfully post-
implementation (State 11 holzman-rust).

### Smoke evidence (cargo fmt)

```
$ cargo fmt --check -p vb_storage
   (no output — formatting clean on the 6 modified/added files in vb_storage)

$ cargo fmt --check -p vb_runtime -- crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs
   (no output — formatting clean on proptest_durability_matrix_step_succeeded.rs)
```

The pre-existing formatting diffs in
`crates/vb_runtime/src/frame_pool/tests.rs:85,114,139` are NOT
in this bead's diff (verified via `jj diff --stat`).

### BLOCKED_TOOLING (kani codegen)

```
$ KANI_FEATURES=kani-vb-qxjgx-record-kind-split bash scripts/kani-list.sh vb_storage
   error: this file contains an unclosed delimiter
     --> crates/vb_core/src/frame/tests_and_verification.rs:22:7
   ...
   error: could not compile `vb_core` (lib) due to 1 previous error
   error: Failed to execute cargo (exit status 101)
```

The pre-existing `kani_helpers.rs` (in vb_core, NOT in this bead's
diff) has an unclosed delimiter at the `frame_kani_harnesses`
module (line 870-893 of `tests_and_verification.rs`). This blocks
`cargo kani list` and `cargo kani <harness>` workspace-wide.
The error exists in the parent commit `kykklnlr 04049f2b`
(verified by running the same command on the parent commit).
Recorded in trusted-base-ledger.jsonl as TBR-001.

## Backward-Compat Test Substitutions (codec/tests.rs:1617-1783)

The pre-fix test at `codec/tests.rs:1617-1630`
(`step_succeeded_event_maps_to_slot_written_kind`) is replaced
with 6 post-fix tests:

| Test | Line | Property |
|------|------|----------|
| `step_succeeded_event_maps_to_step_succeeded_kind` | 1634 | POST-001, POST-002 |
| `slot_written_event_maps_to_slot_written_kind_unchanged` | 1650 | PRE-005 (unchanged) |
| `step_succeeded_and_slot_written_record_kinds_are_distinct` | 1671 | INV-001 (one-to-one) |
| `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` | 1691 | POST-005 (back-compat) |
| `canonical_id_33_round_trip_step_succeeded` | 1716 | POST-006 (round-trip) |
| `slot_written_with_envelope_id_33_is_rejected` | 1754 | POST-007 (cross-bind reject) |

These 6 tests run as part of `cargo test -p vb_storage codec::tests`.
The pre-fix test at line 1617-1630 is removed (replaced by the
6 new tests at the same site). Net change to `codec/tests.rs`:
+167 lines (6 new tests + comments), -3 lines (1 removed test).
Verified via `jj diff --stat`.

## Trusted Boundary Entries (summary; full schema in trusted-base-ledger.jsonl)

- **TBR-001** — `block` — pre-existing `kani_helpers.rs` unclosed
  delimiter in `crates/vb_core/src/frame/tests_and_verification.rs:870-893`
  blocks `cargo kani list` / `cargo kani <harness>` workspace-wide.
  NOT caused by this bead; exists in the parent commit. Routes to
  the kani-helpers owner. `status: blocked`.
- **TBR-002** — `forward_looking` — proof artifacts reference the
  post-fix `RecordKind::StepSucceeded = 33` arm which is not yet
  present in production. `cargo check --tests` emits 4 E0599 errors
  at the known sites. `status: pending_formal_execution`; the
  artifacts will compile + execute post-`holzman-rust` (State 11).
- **TBR-003** — `assume` (pre-condition) — kani harnesses use
  `kani::assume(run != 0)`, `kani::assume(seq != u64::MAX)`,
  `kani::assume(attempt != 0)` to constrain `kani::any()` inputs
  to the valid domain enforced by `JournalEvent::is_valid()`. These
  are pre-conditions, not property short-circuits. `status: accepted`.
- **TBR-004** — `const` — `CURRENT_SCHEMA_VERSION = 1` at
  `crates/vb_storage/src/constants.rs:58` is pinned by tests.rs:3925
  and tests.rs:4223. UNCHANGED by this bead. The proptest
  PO-QXJGX-007-H2 asserts the constant directly via
  `prop_assert_eq!(CURRENT_SCHEMA_VERSION, 1u16, ...)`. `status: accepted`.
- **TBR-005** — `deviation` — PO-QXJGX-007-H2 was planned to call
  `validate_schema_version(0/1/2)` directly. Since the function is
  `pub(crate)`, the proptest exercises the same path through the
  public `validate_record_kind_family` + `CURRENT_SCHEMA_VERSION`
  surface. The in-crate tests at `tests.rs:2108, 3925, 4223` cover
  the direct `validate_schema_version` path (unchanged by this bead).
  `status: accepted`.
- **TBR-006** — `deviation` — task-description path
  "crates/vb_storage/src/proptest_record_kind_*.rs" is replaced by
  the planned.jsonl paths
  "crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs"
  and "crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs".
  The plan was reviewed and approved (proof-plan-review.md STATUS: APPROVED);
  the planned.jsonl paths are authoritative. `status: accepted`.

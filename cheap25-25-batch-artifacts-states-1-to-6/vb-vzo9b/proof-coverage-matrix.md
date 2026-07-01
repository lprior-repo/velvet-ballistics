# Proof Coverage Matrix: vb-vzo9b

## Requirement-to-Obligation Traceability

| Requirement | Contract Clause | Proof Seed | Risk Class | PO-001 (cargo-test summarize_recovery_events) | PO-002 (cargo-test recover_runtime_frame_seed_from_events) | PO-003 (cargo-build + source-lint) | Behavior Affecting |
|-------------|-----------------|------------|------------|------------------------------------------------|--------------------------------------------------------------|-------------------------------------|--------------------|
| VB-VZO9B | C-1 (Exactness of pin, all 11 fields) | PS-vb-vzo9b-1 | equality / field_sensitivity | ✅ primary (covers production derivation stability) | — | ✅ (compile gate proves `assert_eq!` over `RecoveryRuntimeSummary` derives compile) | false |
| VB-VZO9B | C-2 (Sentinel rejection of `RunId::new(0)` in non-empty branch) | PS-vb-vzo9b-2 | equality / sentinel-collision | ✅ transitive (covered by exact-pin: any `RunId` mismatch including `RunId(0)` fails the field-by-field comparison) | — | — | false |
| VB-VZO9B | C-3 (Empty-events path unchanged: `assert_typed_recovery_error`) | PS-vb-vzo9b-3 | rejection / typed-error-rail | ✅ transitive (the empty-events branch is unaffected by the fuzz-body rewrite; existing recovery_unit_tests exercise `RecoveryError::NoRecoveryData { run: RunId::new(0) }`) | — | — | false |
| VB-VZO9B | C-4 (Frame-seed call site unchanged at lines 201-203) | PS-vb-vzo9b-6 | noop | — | ✅ primary (cargo test on `recover_runtime_frame_seed_from_events` confirms the function's contract is preserved; the test calls the same function via the same error sink) | — | false |
| VB-VZO9B | C-5 (No production-code change) | PS-vb-vzo9b-4 | blast-radius-control | ✅ transitive (cargo test gates remain green iff production is unchanged) | ✅ transitive | ✅ primary (the grep + build prove diff is restricted to `readback.rs`) | false |
| VB-VZO9B | C-6 (No new error variant, no new type, no `unsafe`, no `unwrap`/`expect`/`panic` outside desired `assert_eq!`) | PS-vb-vzo9b-4 | source-lint | — | — | ✅ primary (compile gate rejects `unsafe`/`unwrap`/`expect` outside the desired assertion; forbidden-pattern grep catches reintroduction) | false |
| VB-VZO9B | C-7 (Closure commands: `cargo build -p fuzz --bin recovery_decode`, two `cargo test` invocations) | PS-vb-vzo9b-4 | blast-radius-control | ✅ primary | ✅ primary | ✅ primary | false |
| VB-VZO9B | C-8 (Forbidden patterns: no `assert!(* \|\| *)`, no `matches!(*, ..)` over a single field, no field-by-field `assert!` chain, no `let _summary`, no `dbg!`, no `unwrap`/`expect` on `RecoveryResult`) | PS-vb-vzo9b-5 | forbidden-pattern | — | — | ✅ primary (forbidden-pattern grep over `readback.rs`) | false |

## Coverage Legend

- **✅ primary**: This obligation's evidence is the primary satisfaction for the clause.
- **✅ transitive**: This obligation's evidence transitively satisfies the clause as a side-effect of covering another clause's primary claim.
- **—**: Not the primary or transitive carrier; the clause is covered elsewhere.
- **PO-XXX**: Planned proof obligation ID; `status: planned`, `owner_state: 4`.

## Clause → Obligation Mapping

| Clause | Primary Carrier(s) | Transitive Carrier(s) |
|--------|--------------------|------------------------|
| C-1 | PO-001 (cargo-test `summarize_recovery_events`), PO-003 (cargo-build compile of `assert_eq!`) | — |
| C-2 | PO-001 (transitive: sentinel `RunId::new(0)` is one of the 11 fields compared by `assert_eq!`) | — |
| C-3 | PO-001 (transitive: existing recovery_unit_tests cover the empty-events `RecoveryError::NoRecoveryData` rail) | — |
| C-4 | PO-002 (cargo-test `recover_runtime_frame_seed_from_events`) | — |
| C-5 | PO-003 (cargo-build + grep scope restriction) | PO-001, PO-002 |
| C-6 | PO-003 (compile rejects `unsafe`/`unwrap`/`expect`; forbidden-pattern grep) | — |
| C-7 | PO-001, PO-002, PO-003 | — |
| C-8 | PO-003 (forbidden-pattern grep) | — |

## Obligation → Verifier Lane Pairing

| Obligation ID | Verifier | Lane Decision | Status |
|---------------|----------|---------------|--------|
| PO-001 | proptest (cargo-test) | VLD-001 (required) | planned |
| PO-002 | proptest (cargo-test) | VLD-002 (required) | planned |
| PO-003 | proptest (cargo-build + source-lint) | VLD-003 (required) | planned |

## Total Counts

- Proof Seeds: 6 (`PS-vb-vzo9b-1..6`)
- Required Proof Obligations: **3** (PO-001, PO-002, PO-003)
- Default-profile Verifiers: 0 required (verus, kani, flux-rs, loom, miri, cargo-fuzz all `not_applicable`)
- Waiver Candidates: 0 (no behavior-affecting waivers; only one structural placeholder row)
- Trusted Base Entries: 0 (no trust markers introduced)

## Self-Audit Checklist

- [x] Every `(requirement_id, contract_clause)` tuple has at least one primary or transitive carrier obligation.
- [x] Every required lane decision has at least one paired `proof-obligation/v1` ID, and the obligation exists in `proof-obligations.planned.jsonl`.
- [x] No `behavior_affecting: true` obligations (test-only repair).
- [x] No waivers cover production behavior.
- [x] All obligations have absolute `workdir`, exact `command`, and concrete `expected_evidence` markers.
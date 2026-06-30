# Formal Verification Report — vb-rpch Flux r9

bead: `vb-rpch`  
state: 12 formal execution — Flux r9 sublane only  
workdir: `/home/lewis/src/vb-jpq7-jj-fix`  
date: 2026-05-24  
skills invoked: `flux-rs`, `formal-verifier`

## Scope

This report closes only the approved Flux r9 sublane for `verification/flux/vb_rpch_flux_r9.rs` at scoped single-file harness level. It does not claim production-body Flux verification, full crate-mode Flux verification, Kani closure, proptest/fuzz closure, or Rust-attachment closure.

Approved inputs consumed:

- `.beads/vb-rpch/proof-review-flux-r9.md`
- `.beads/vb-rpch/proof-evidence-flux-r9.md`
- `.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl`
- `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl`
- `verification/flux/vb_rpch_flux_r9.rs`

## Commands executed

All commands were executed from `/home/lewis/src/vb-jpq7-jj-fix`.

| Command | Result |
| --- | --- |
| `z3 --version` | PASS; exit 0; `Z3 version 4.16.0 - 64 bit` |
| `fixpoint --version` | PASS; exit 0; `fixpoint 0.9.6.3.6 (6f214fd7a67c1e61f3f165569b88dfdec2dda0d9)` |
| `flux --version` | PASS; exit 0; `flux 4d329f2 (2026-05-23)` |
| `cargo flux -V` | PASS; exit 0; `cargo-flux 4d329f2 (2026-05-23)` |
| `flux --crate-type lib --edition 2024 verification/flux/vb_rpch_flux_r9.rs` | PASS; exit 0; `summary. 50 functions processed: 50 checked; 0 trusted; 0 ignored. 38 constraints solved. Finished in 242.58ms` |
| `/usr/bin/rg -n '#!?\[(flux_rs::\|flux::)?(trusted\|trusted_impl\|extern_spec\|ignore\|no_panic\|no_panic_if)(\([^]]*\))?\]\|unsafe' --glob '*.rs' --glob '!**/target/**' verification/flux/vb_rpch_flux_r9.rs crates/vb_storage/src/recovery/types.rs crates/vb_storage/src/recovery/hydrate.rs crates/vb_storage/src/recovery/replay/core.rs crates/vb_storage/src/recovery/replay/summary.rs` | PASS; exit 0; matches were only `#![forbid(unsafe_code)]` in mapped files |
| JSONL validation for `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl` and `.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl` | PASS; exit 0; 5 trusted-base records and 7 proof-obligation records parsed as JSONL |

## Trusted-boundary scan result

Scan output:

```text
crates/vb_storage/src/recovery/replay/core.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/types.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/replay/summary.rs:1:#![forbid(unsafe_code)]
verification/flux/vb_rpch_flux_r9.rs:2:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/hydrate.rs:1:#![forbid(unsafe_code)]
```

No `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, `#[ignore]`, `#[no_panic]`, `#[no_panic_if]`, or executable `unsafe` markers were found in the scoped r9 harness and mapped recovery source files. The Flux checker also reported `0 trusted; 0 ignored` for the single-file harness.

## JSONL validation

```text
.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl: 5 jsonl records valid; ids=['TB-R9-FLUX-SCOPED-SINGLE-FILE-HARNESS', 'TB-R9-FLUX-PRODUCTION-CORRESPONDENCE', 'TB-R9-FLUX-HASHSET-MEMBERSHIP-ABSTRACTION', 'TB-R9-FLUX-CRATE-MODE-ICE', 'TB-R9-FLUX-NO-TRUSTED-IGNORE-MARKERS']
.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl: 7 jsonl records valid; ids=['VFR-R2-FLUX-001', 'VFR-R2-FLUX-002', 'VFR-R2-FLUX-003', 'VFR-R2-FLUX-004', 'VFR-R2-FLUX-005', 'VFR-R2-FLUX-006', 'VFR-R2-FLUX-007']
```

## Obligation closure

The following are marked `PASS` only at scoped single-file Flux harness level:

- `VFR-R2-FLUX-001` — PASS scoped single-file harness.
- `VFR-R2-FLUX-002` — PASS scoped single-file harness.
- `VFR-R2-FLUX-003` — PASS scoped public-membership surface only; production `HashSet` behavior remains outside Flux r9.
- `VFR-R2-FLUX-004` — PASS scoped single-file harness.
- `VFR-R2-FLUX-005` — PASS scoped precondition surface only; snapshot byte decodability and event iteration remain outside Flux r9.
- `VFR-R2-FLUX-006` — PASS scoped precondition surface only.
- `VFR-R2-FLUX-007` — PASS scoped pure replay surface only; full replay loop behavior remains outside Flux r9.

## Residual limitations and blockers preserved

- No production-body Flux verification is claimed.
- No full `vb_storage` crate-mode Flux verification is claimed; prior evidence records a Flux internal compiler error when full crate metadata was enabled.
- `TB-R9-FLUX-SCOPED-SINGLE-FILE-HARNESS`, `TB-R9-FLUX-PRODUCTION-CORRESPONDENCE`, `TB-R9-FLUX-HASHSET-MEMBERSHIP-ABSTRACTION`, and `TB-R9-FLUX-CRATE-MODE-ICE` remain active limitations.
- `VFR-R2-KANI-005..007` remain open outside this Flux sublane.
- `VFR-R2-RUST-ATTACH-001..007` remain open outside this Flux sublane.
- proptest/fuzz/non-Flux blockers are not closed by this report.
- Prior provenance limitation from `.beads/vb-rpch/proof-review-flux-r9.md` is preserved for final/full-gate closure.

STATUS: PASS — Flux r9 sublane only, scoped single-file harness level.

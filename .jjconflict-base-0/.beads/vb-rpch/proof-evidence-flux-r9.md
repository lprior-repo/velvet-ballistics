# Proof Evidence — vb-rpch Flux r9

bead: `vb-rpch`  
state: 5 proof/model/harness repair — Flux RS r9  
workdir: `/home/lewis/src/vb-jpq7-jj-fix`  
date: 2026-05-24

## Tool discovery

```sh
z3 --version
```

Result: exit 0

```text
Z3 version 4.16.0 - 64 bit
```

```sh
if command -v fixpoint >/dev/null; then fixpoint --version; else liquid-fixpoint --version; fi
```

Result: exit 0

```text
Liquid-Fixpoint Copyright 2009-25 Regents of the University of California.
All Rights Reserved.

fixpoint 0.9.6.3.6 (6f214fd7a67c1e61f3f165569b88dfdec2dda0d9)
```

```sh
flux --version
```

Result: exit 0

```text
flux 4d329f2 (2026-05-23)
```

```sh
cargo flux -V
```

Result: exit 0

```text
cargo-flux 4d329f2 (2026-05-23)
```

## Flux claim command

```sh
flux --crate-type lib --edition 2024 "verification/flux/vb_rpch_flux_r9.rs"
```

Result: exit 0

```text
summary. 50 functions processed: 50 checked; 0 trusted; 0 ignored. 38 constraints solved. Finished in 254.40ms
```

Interpretation: r9 scoped Flux artifact verifies with no trusted or ignored functions. The file contains `#[should_fail]` negative checks; command success means the local Flux checker observed those expected failures.

## Crate-mode metadata probe

I temporarily tried minimal non-behavioral package metadata:

```toml
[package.metadata.flux]
enabled = true
```

Then ran:

```sh
cargo flux -p vb_storage --message-format human
```

Result: exit 101

```text
    Checking vb_storage v0.1.0 (/home/lewis/src/vb-jpq7-jj-fix/crates/vb_storage)
error: internal compiler error: crates/flux-infer/src/projections.rs:382:13: impossible case reached
   --> crates/vb_storage/src/admission.rs:270:17
    |
270 |       let keyed = action_contracts
    |  _________________^
271 | |         .iter()
272 | |         .filter(|contract| requires_idempotency_key(contract))
273 | |         .map(|contract| contract.id)
    | |____________________________________^

thread 'rustc' (3523013) panicked at crates/flux-infer/src/projections.rs:382:13:
Box<dyn Any>
error: could not compile `vb_storage` (lib)
```

Disposition: metadata was removed/restored. Full crate-mode Flux adoption is not safe/minimal in this sublane because it trips a Flux internal compiler error in unrelated admission iterator/projection code before reaching recovery proofs.

After removing the metadata, I reran the smoke command:

```sh
cargo flux -p vb_storage --message-format human
```

Result: exit 0

```text
    Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.05s
```

Interpretation: crate driver/tooling is present, but without package metadata it is not property evidence for the recovery helpers. The scoped single-file r9 artifact remains the accepted Flux evidence.

## Trusted-boundary scan

```sh
/usr/bin/rg -n '#!?\[(flux_rs::|flux::)?(trusted|trusted_impl|extern_spec|ignore|no_panic|no_panic_if)(\([^]]*\))?\]|unsafe' --glob '*.rs' --glob '!**/target/**' "verification/flux" "crates/vb_storage/src/recovery/types.rs" "crates/vb_storage/src/recovery/hydrate.rs" "crates/vb_storage/src/recovery/replay/core.rs" "crates/vb_storage/src/recovery/replay/summary.rs"
```

Result: exit 0

```text
crates/vb_storage/src/recovery/hydrate.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/replay/summary.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/types.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/replay/core.rs:1:#![forbid(unsafe_code)]
verification/flux/vb_rpch_flux_r9.rs:2:#![forbid(unsafe_code)]
verification/flux/vb_rpch_flux_r8.rs:2:#![forbid(unsafe_code)]
```

Interpretation: no `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, `#[ignore]`, `#[no_panic]`, or `#[no_panic_if]` surfaces in the r9 harness or mapped recovery sources. Matches are only `#![forbid(unsafe_code)]`.

## JSONL validation

```sh
python3 - <<'PY'
import json
from pathlib import Path
for path in [
    Path('.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl'),
    Path('.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl'),
]:
    count = 0
    for lineno, line in enumerate(path.read_text().splitlines(), 1):
        if line.strip():
            json.loads(line)
            count += 1
    print(f'{path}: {count} jsonl records valid')
PY
```

Result: exit 0

```text
.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl: 5 jsonl records valid
.beads/vb-rpch/proof-obligations.flux-r9.written.jsonl: 7 jsonl records valid
```

## Properties proved

- `VFR-R2-FLUX-001`: `UnsupportedRecoveryState` supported constant is all false; union returns field-wise OR; `union_matches_flags_surface` checks production helper semantics. Negative checks reject a true field in `SUPPORTED` and reject a mismatched union.
- `VFR-R2-FLUX-002`: present dimension index `< 65535` produces positive count; absent observation produces zero; observed-dimension predicate matches presence; positive seed dimensions imply the public positive predicate. Negative checks reject zero seed dimension and absent observation with positive count.
- `VFR-R2-FLUX-003`: tracker mark operations expose refined post-states (`completed=true` or `failed=true`); `is_resolved` return type is `completed || failed`; monotonic proofs now return `is_resolved_surface(tracker)` directly from refined return types, not `A || B`. Negative check rejects a new unresolved tracker.
- `VFR-R2-FLUX-004`: digest hierarchy ranks `1 < 2 < 3`, check-level predicates, and source-only-not-full negative.
- `VFR-R2-FLUX-005`: snapshot-tail precondition is the conjunction of run match, sequence after snapshot, and evidence; positive hydrate dimensions are proved for positive step/slot counts; negative missing evidence rejected.
- `VFR-R2-FLUX-006`: events-only precondition requires non-empty events; negative empty event length rejected.
- `VFR-R2-FLUX-007`: stale attempt implies not current; stale-state-effect is state-effect AND stale; decreasing step order implies divergence; negative state-effect-without-stale and increasing-step-order calls rejected.

## Bounds and assumptions

- u16 dimension arithmetic is bounded by `max_index < 65535` before `+1`, matching the production checked-add/overflow shape.
- Replay attempt/step surfaces use u16 values, matching the production value domains exposed by `attempt` and `StepIdx::get()`.
- `ActionReplayTracker` HashSet membership is represented by booleans for one action/step pair; std `HashSet` itself is not Flux-proved.
- Full `JournalEvent` iteration, snapshot byte decoding, vector output preservation, and replay loop effects remain outside this Flux sublane.

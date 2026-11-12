# Proof Evidence — vb-rpch verus-flux-rust-r8 Flux RS

bead: `vb-rpch`  
state: 5 proof/model/harness writing — Flux RS lane after tooling install  
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

## Help/syntax discovery

```sh
flux --help
cargo flux --help
```

Result: exit 0 for both commands. `flux --help` reports rustc-driver style single-file options including `--crate-type` and `--edition`. `cargo flux --help` reports package selection, message-format, features, manifest options, `check` default command, and `-V/--version`.

Local scratch syntax probes were run under `/tmp/opencode` before writing the harness. They validated current local syntax for `#[spec]`, `#[refined_by]`, `#[field]`, `#[variant]`, `#[should_fail]`, boolean indexed values, refined enums, and u16 arithmetic refinements. Scratch files are not proof artifacts.

## Flux claim commands

### Exact planned crate Flux command

```sh
cargo flux -p vb_storage --message-format human
```

Result: exit 0

```text
    Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.06s
```

Interpretation: tooling and crate-driver pass for `vb_storage`. This is **not** counted as property evidence by itself because the production crate has no r8 Flux refinements for the target recovery helpers.

### Scoped Flux harness command

```sh
flux --crate-type lib --edition 2024 "verification/flux/vb_rpch_flux_r8.rs"
```

Result: exit 0

```text
summary. 37 functions processed: 37 checked; 0 trusted; 0 ignored. 24 constraints solved. Finished in 141.63ms
```

Interpretation: r8 Flux claim evidence for the scoped harness. No `#[trusted]` or `#[ignore]` bodies were used. `#[should_fail]` negative obligations are included in the 37 checked functions and the command exits successfully only because the local Flux checker observes the expected failures.

## Trusted-boundary scan

```sh
/usr/bin/rg -n '#!?\[(flux_rs::|flux::)?(trusted|trusted_impl|extern_spec|ignore|no_panic|no_panic_if)(\([^]]*\))?\]|unsafe' --glob '*.rs' --glob '!**/target/**' "verification/flux" "crates/vb_storage/src/recovery/types.rs" "crates/vb_storage/src/recovery/hydrate.rs" "crates/vb_storage/src/recovery/replay/core.rs" "crates/vb_storage/src/recovery/replay/summary.rs"
```

Result: exit 0

```text
crates/vb_storage/src/recovery/replay/core.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/replay/summary.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/types.rs:1:#![forbid(unsafe_code)]
crates/vb_storage/src/recovery/hydrate.rs:1:#![forbid(unsafe_code)]
verification/flux/vb_rpch_flux_r8.rs:2:#![forbid(unsafe_code)]
```

Interpretation: no Flux `trusted`, `trusted_impl`, `extern_spec`, `ignore`, `no_panic`, or `no_panic_if` markers in the verified Flux harness or mapped production pure-surface files. Matches are only `#![forbid(unsafe_code)]`.

## JSONL validation

```sh
python3 - <<'PY'
import json
from pathlib import Path
for path in [
    Path('.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r8.jsonl'),
    Path('.beads/vb-rpch/proof-obligations.verus-flux-rust-r8.written.jsonl'),
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
.beads/vb-rpch/trusted-base-ledger.verus-flux-rust-r8.jsonl: 4 jsonl records valid
.beads/vb-rpch/proof-obligations.verus-flux-rust-r8.written.jsonl: 7 jsonl records valid
```

## Properties actually proved

- `VFR-R2-FLUX-001`: Flux refined `UnsupportedRecoveryState` by four boolean fields; proved the supported constructor has all fields false; proved union returns field-wise OR; included a `#[should_fail]` negative constructor that tries to return `SUPPORTED` with `slot_values = true`.
- `VFR-R2-FLUX-002`: proved the present-index checked helper maps `u16 < 65535` to a positive `u16` count by `+1`; proved positive checked seed dimensions imply the public positive-dimension predicate; included `#[should_fail]` negative zero-dimension call.
- `VFR-R2-FLUX-003`: proved the pure ActionReplayTracker support surface: completed/failed booleans are monotone after mark operations and `is_resolved == completed || failed`. Production `HashSet` behavior is explicitly not proved by Flux r8.
- `VFR-R2-FLUX-004`: proved the `DigestCheck` rank hierarchy `1 < 2 < 3`, check-level predicates, and a negative source-only/full-check rejection.
- `VFR-R2-FLUX-005`: proved snapshot-tail preconditions are the conjunction of run-match, sequence-after-snapshot, and evidence-present pure facts.
- `VFR-R2-FLUX-006`: proved events-only hydrate precondition requires non-empty events length.
- `VFR-R2-FLUX-007`: proved pure replay precondition facts: stale attempt implies not current, stale-state-effect is state-effect AND stale, and decreasing step order implies divergence. Full replay loop behavior is residual.

## Assumptions, bounds, and limitations

- Bounds: u16 dimension proof uses `max_index < 65535` to avoid overflow and prove `max_index + 1 > 0`.
- Bounds: replay attempt and step-order pure predicates use u16 surfaces matching the production index/attempt value domains.
- Scope: single-file Flux harness, not full crate body verification.
- Source correspondence: harness mirrors named production pure helpers by inspection; no mechanical import/link to production functions is claimed.
- Data structure limitation: production `HashSet` insertion/membership semantics are abstracted for `ActionReplayTracker`.
- Hydrate/replay limitation: JournalEvent iteration, snapshot byte decoding, vector output preservation, and replay loop effects are not Flux-proved in r8.
- No broad `#[trusted]`, `#[extern_spec]`, `#[ignore]`, or crate-wide Flux skip was added.

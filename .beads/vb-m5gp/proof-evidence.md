# Proof Evidence: vb-m5gp

## Obligation Coverage

### PO-014 — Kani idempotency parity

- Artifact: `crates/vb_compile/src/kani_idempotency_parity.rs`
- Supporting artifacts repaired: `crates/vb_validate/src/kani_gate_08_accessor.rs`, `crates/vb_validate/src/kani_gate_08_structural.rs`
- Command: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`
- Final result: PASS, exit 0.

## Exact Evidence

### Workspace guard

Command:

```bash
pwd -P && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -s .beads/vb-m5gp/contract.md && test -s .beads/vb-m5gp/traceability-matrix.jsonl && test -s .beads/vb-m5gp/delivery-scope.jsonl
```

Output:

```text
/home/lewis/src/go-skill-vb-m5gp
```

### Planned obligation validation

Command:

```bash
python -c 'import json, pathlib; p=pathlib.Path(".beads/vb-m5gp/proof-obligations.planned.jsonl"); required=["id","requirement_id","contract_clause","risk","verifier","artifact","command","expected_evidence","assumptions","required","mode","owner_state","rerun_from","status"]; rows=[json.loads(line) for line in p.read_text().splitlines() if line.strip()]; missing=[(r.get("id"), [k for k in required if k not in r]) for r in rows if any(k not in r for k in required)]; print(f"rows={len(rows)} missing={missing}"); raise SystemExit(1 if missing else 0)'
```

Output:

```text
rows=20 missing=[]
```

### Tool discovery

Command:

```bash
cargo kani --version
```

Output:

```text
cargo-kani 0.67.0
```

### Initial Kani failure

Command:

```bash
cargo kani --package vb_compile --harness idempotency_gate_parity --quiet
```

Result: FAIL, exit 101.

Relevant output:

```text
error[E0004]: non-exhaustive patterns: `&_` not covered
   --> crates/vb_validate/src/kani_gate_08_accessor.rs:24:19
...
error[E0004]: non-exhaustive patterns: `&_` not covered
   --> crates/vb_validate/src/kani_gate_08_structural.rs:34:19
...
error: Failed to execute cargo (exit status: 101). Found 2 compilation errors.
```

### Final Kani pass

Command:

```bash
cargo kani --package vb_compile --harness idempotency_gate_parity --quiet
```

Output:

```text
   Compiling vb_validate v0.1.0 (/home/lewis/src/go-skill-vb-m5gp/crates/vb_validate)
   Compiling vb_compile v0.1.0 (/home/lewis/src/go-skill-vb-m5gp/crates/vb_compile)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.99s
```

Result: PASS, exit 0.

### Formatting

Command:

```bash
cargo +nightly fmt --all --check
```

Output: none.

Result: PASS, exit 0.

## Assumptions, Bounds, and Trusted Boundaries

- Kani version: `cargo-kani 0.67.0`.
- PO-014 harness bound: `#[kani::unwind(8)]` in `idempotency_gate_parity`.
- PO-014 decision-table bound: 5 `SideEffect` variants × 3 `RetrySafety` variants × 3 `Idempotency` variants = 45 cases.
- Fixed `ActionContract` fields in PO-014 (`ActionId::new(0)`, slot counts, byte limits, timeout, empty capabilities) are outside the idempotency decision variables and serve as irrelevant witness fields for the two compared APIs.
- Supporting Gate 8 Kani harnesses use `#[kani::unwind(5)]` where already present.
- New PO-014 support assumption: in bounded-valid-accessor Gate 8 harnesses, wildcard/future `PathSegment` variants are excluded with `kani::assume(false)` because `PathSegment` is non-exhaustive and the harness domain covers current `Field` and `Index` variants only.
- No production implementation behavior, dependency, feature, or config files were edited.

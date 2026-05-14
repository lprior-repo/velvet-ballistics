# Verus Evidence: vb-gvmt

## Command

```bash
/home/lewis/.local/bin/verus --crate-type=lib ".beads/vb-gvmt/proofs/generated_semantics_verus.rs"
```

## Result

- Status: PASS
- Observed evidence: `verification results:: 6 verified, 0 errors`

## Tooling Caveat

Bare `verus --crate-type=lib ...` failed in this environment with `error: did not find a valid verusroot` because `verus` resolved to a broken wrapper under `/home/lewis/.cargo/bin/verus`. The passing evidence intentionally pins `/home/lewis/.local/bin/verus`.

## Scope

Verified obligations in `.beads/vb-gvmt/proofs/generated_semantics_verus.rs` cover checked slot access, parallel slot/taint writes, taint join monotonicity, no-contract clean-output rejection for tainted input, journal capacity append behavior, and resume identity validation before mutation.

# vb-txhe - Add Verus lane to proof gauntlet

STATUS: IMPLEMENTED

## Scope

- Added `scripts/verify-verus.sh` as the fail-closed Verus registry runner.
- Patched `scripts/rust-verification-gauntlet.sh` so `proof` mode runs Verus before Kani and Lean.
- Report plumbing now records Verus summaries in `.evidence/verus/summary.txt` and requires `Verus registry summary` in the formal report validator.

## Evidence

- `bash scripts/verify-verus.sh` -> `VERUS_REGISTRY_OK evidence=.evidence/verus`.
- `bash scripts/rust-verification-gauntlet.sh proof` -> PASS after stashing unrelated local `vb_storage`/CLI refactor edits; Verus all targets PASS, Kani compiles and reports no manual harnesses, Lean skips because no Lean proof directory exists.
- `verusfmt` was unavailable and is recorded as `VERUSFMT=VERUSFMT_MISSING`.

## Trusted boundary

No Verus `assume`, `#[verifier::external_body]`, `#[verifier::external]`, or `axiom` was added. The Verus lane trust scan records `VERUS_TRUST_SCAN_OK`.

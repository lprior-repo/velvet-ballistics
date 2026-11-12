# Machine Gate Report — vb-xkli

STATUS: APPROVED

## Command

`TMPDIR=target/tmp bash scripts/rust-verification-gauntlet.sh proof`

## Result

Exit 0. Script printed `[PASS] All proof checks passed`.

## Kani Passes

- `KANI-EXPR-BYTECODE-001`.
- `KANI-SLOT-REF-001`.
- `KANI-CONSTANT-POOL-001`.
- `KANI-ACCESSOR-REF-001`.
- `INV-007-NODEDUP-001`.
- `KANI-ADMISSION-001-MALFORMED-GATE-PROOF-REJECT`.
- `KANI-ADMISSION-001-CAPABILITY-REJECT`.
- `KANI-ADMISSION-001-VALID-ACCEPT`.

## Limitation

Root `cargo kani list --format json` is not usable in this workspace and reports `No supported targets were found`; scripted harness commands are the raw proof evidence.

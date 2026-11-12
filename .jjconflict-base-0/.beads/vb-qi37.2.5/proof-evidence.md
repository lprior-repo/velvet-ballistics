# Proof Evidence - vb-qi37.2.5 State 5 FUZZ-RESOURCE-001 repair

STATUS: READY_FOR_REVIEW

## Environment

- Timestamp: `2026-05-16T12:34:46Z`.
- Working directory: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Forbidden source checkout for writes: `/home/lewis/src/velvet-ballistics`.
- Boundary: `.beads/vb-qi37.2.5/` proof evidence/report/state refresh only.
- Production edits: none.
- Test edits: none.
- Proof/model/harness edits: none.

## Raw Command Evidence

### Isolation Guard

Command:

```bash
pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac
```

Exit: 0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5
```

### FUZZ-RESOURCE-001 Repaired Stdin Replay Plus Companion Proptest

Command:

```bash
mkdir -p target/tmp && RUSTC_WRAPPER= TMPDIR=target/tmp rtk cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget && python3 -c "import subprocess; from pathlib import Path; t=Path('target/debug/resource_budget'); assert t.exists(), f'missing {t}'; fixed=[b'', b'\x00', b'\x00'*32, b'\xff'*32, b'fanout-over-policy', b'nesting-over-policy', b'compact-step-overflow', b'max-slots-cap-one-over', b'payload-length-header-one-over']; cases=fixed+[(i.to_bytes(8,'little') + bytes([(i*31)%256])*(i%64))[:72] for i in range(991)]; [(_ for _ in ()).throw(SystemExit(f'resource_budget stdin replay failed at case {idx} rc={r.returncode}')) for idx,data in enumerate(cases) for r in [subprocess.run([str(t)], input=data, timeout=2)] if r.returncode != 0]; print(f'resource_budget stdin replay PASS cases={len(cases)}')" && RUSTC_WRAPPER= TMPDIR=target/tmp PROPTEST_CASES=10000 rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial proptest -- --nocapture
```

Exit: 0

```text
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
resource_budget stdin replay PASS cases=1000
cargo test: 3 passed, 19 filtered out (1 suite, 0.11s)
```

Obligation: `PO-009` / `FUZZ-RESOURCE-001`.

## Discharge Decision

- `FUZZ-RESOURCE-001`: PASS for the repaired evidence lane.
- Required evidence present: exact stdout `resource_budget stdin replay PASS cases=1000`.
- Companion evidence present: `PROPTEST_CASES=10000` adversarial proptest reports `3 passed, 19 filtered out`.
- No PASS is claimed for `cargo fuzz run resource_budget -- -runs=1000`.
- The cargo-fuzz command is treated only as invalid evidence for the current stdin-once driver, matching the repaired State 3/4 obligation text.

## Non-Claims

- This is not a libFuzzer coverage result.
- This does not modify or certify production/test source changes.
- This repair does not rerun Verus, TLA+, Miri, source lint, or full State 11 machine gates.

## Anti-Hallucination Notes

- Every PASS claimed above has command output and exit status from this session.
- No production, test, dependency, config, Verus, TLA+, Kani, or fuzz harness files were edited.
- Prior State 5/11 Verus, TLA+, proptest, Miri, and lint evidence remains historical context only; this repair addresses the `FUZZ-RESOURCE-001` failed proof/evidence lane after State 4 plan repair.

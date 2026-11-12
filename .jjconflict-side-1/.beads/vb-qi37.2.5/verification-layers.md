# Verification Layers - vb-qi37.2.5

## Boundary
- Verus-owned kernel: pure step-budget and resource-budget arithmetic.
- TLA+ temporal model: finite execution slices, exhaustion, admission/rejection, and value-growth terminal outcomes.
- Theorem projection: none; Lean waived with Verus/TLA+ compensating evidence.
- Runtime shell: `run_until_blocked`, `drive_deterministic`, `ValueStore`, `ResourceContract::validate`, `WholeWorkflowBudget::compute`.
- External systems excluded from formal proof: `vb_runtime` generated chunks, filesystem/storage, action handlers, OS memory exhaustion.

## Layer Assignment
- PRE-001 -> integration scenario + mutation review.
- PRE-002 -> proptest bounded generator review + fuzz bounded-input obligation.
- PRE-003 -> unit/integration scenario + Kani/proptest where harness exists.
- PRE-004 -> integration scenario + Miri + proptest.
- PRE-005 -> integration scenario + TLA+ admission model + Verus resource budget lemmas.
- PRE-006 -> formal-verifier classification check; `DEFERRED_GLOBAL` evidence only.
- POST-001 -> TLA+ + engine integration scenario + Kani/proptest.
- POST-002 -> budget policy unit scenario + mutation.
- POST-003 -> budget policy unit scenario + mutation.
- POST-004 -> value-store integration scenario + Miri + proptest.
- POST-005 -> value-store limit scenarios + fuzz.
- POST-006 -> TLA+ admission model + Verus resource-budget lemmas + nested verifier scenario.
- POST-007 -> Verus + proptest.
- POST-008 -> fuzz + panic/static scan + mutation.
- INV-001 -> Verus + proptest.
- INV-002 -> TLA+ + Kani/proptest + engine integration scenario.
- INV-003 -> proptest + Miri + mutation.
- INV-004 -> proptest + Kani/fuzz where harness exists.
- INV-005 -> unit scenarios + mutation.
- INV-006 -> Verus + TLA+ + proptest.
- INV-007 -> unit/source invariant scenarios + static scan.
- INV-008 -> deterministic stdin replay over 1000 bounded hostile inputs + focused malformed-byte/property tests; cargo-fuzz `-runs=1000` is waived as invalid evidence for the current stdin-once driver.

## Verus Scope
- `verification/verus/step_budget.rs`: proves `remaining_after_take` underflow freedom, preservation on failure, monotonicity, and exact zero behavior.
- `verification/verus/resource_budget.rs`: proves boundedness of saturated add/multiply, sequential/branch/loop composition, and policy preservation.
- Exact commands:
  - `verus verification/verus/step_budget.rs`
  - `verus verification/verus/resource_budget.rs`
- Trusted boundary: Rust implementation refines these spec functions through validated constructors and private fields; Verus does not execute runtime shell or allocate stores.
- Shell exclusions: I/O, async scheduling, storage, generated runtime, fuzz harnesses, test fixture allocation.

## TLA+ Scope
- Model paths: `specs/vb_qi37_2_5/BoundednessSlice.tla` and `specs/vb_qi37_2_5/NestedBoundednessAdmission.tla`.
- Variables/actions/properties are defined in `tla-spec.md`.
- Evidence commands:
  - `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-slice specs/vb_qi37_2_5/BoundednessSlice.tla -config specs/vb_qi37_2_5/BoundednessSlice.cfg`
  - `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-nested specs/vb_qi37_2_5/NestedBoundednessAdmission.tla -config specs/vb_qi37_2_5/NestedBoundednessAdmission.cfg`

## Kani Scope
- Existing relevant harness files discovered: `kani/gate_11_loop.rs`, `kani/gate_12_14_15.rs`.
- Waiver: `KANI-LOOP-001` is waived for State 3 contract adequacy because the discovered files are standalone `kani/` artifacts and are not Cargo-integrated workspace targets; there is no truthful `cargo kani --package ... --harness ...` command for those files without proof-source or Cargo manifest edits, which are outside this repair scope.
- Limitation: Kani does not discharge POST-001/INV-002 for this repair.
- Compensating evidence: `VERUS-STEP-001` proves step budget monotonicity/underflow freedom, `TLA-SLICE-001` model-checks finite execution-slice eventual exhaustion/block/finish behavior, and downstream exact tests/proptests cover step-budget and boundedness paths.
- Expiry: waiver expires when a Cargo-integrated Kani harness exists for the bounded run-loop/gate property or by the next proof-writing state touching Kani harnesses.

## Fuzz Scope
- Existing target: `fuzz/src/bin/resource_budget.rs` with shared body in `fuzz/src/lib.rs`.
- Invalid command waiver: `cargo fuzz run resource_budget -- -runs=1000` is not valid evidence for this obligation because the local cargo-fuzz default selects static musl with ASAN and the current target is a stdin-once driver, not a true libFuzzer harness.
- Exact repaired stdin replay command: `mkdir -p target/tmp && RUSTC_WRAPPER= TMPDIR=target/tmp cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget && python3 -c "import subprocess; from pathlib import Path; t=Path('target/debug/resource_budget'); assert t.exists(), f'missing {t}'; fixed=[b'', b'\x00', b'\x00'*32, b'\xff'*32, b'fanout-over-policy', b'nesting-over-policy', b'compact-step-overflow', b'max-slots-cap-one-over', b'payload-length-header-one-over']; cases=fixed+[(i.to_bytes(8,'little') + bytes([(i*31)%256])*(i%64))[:72] for i in range(991)]; [(_ for _ in ()).throw(SystemExit(f'resource_budget stdin replay failed at case {idx} rc={r.returncode}')) for idx,data in enumerate(cases) for r in [subprocess.run([str(t)], input=data, timeout=2)] if r.returncode != 0]; print(f'resource_budget stdin replay PASS cases={len(cases)}')"`.
- Companion commands: focused `vb_qi37_2_5_boundedness_adversarial` execution and `PROPTEST_CASES=10000` proptest filter from `test-plan.md` / `test-writer-report.md`.
- Required claim: malformed resource-budget inputs remain bounded and do not panic, OOM, timeout, or require process kill.

## Miri Scope
- Existing Moon Miri task runs selected lib tests, but not yet value-store cap-specific tests.
- Exact existing command: `moon run :miri`.
- Required future command if a cap-specific test is added: run Miri for that exact test name and record stdout.

## Static/Mutation/Performance
- Static source gate: `moon run :lint-src` or `moon run :quick` after implementation; tests are not source-lint strict by contract.
- Mutation gate: exact `cargo mutants` command is blocked until test-writer names changed test files.
- Performance: no speedup claim in this bead. Only bounded-time/no-timeout evidence is required; no benchmark threshold is asserted.

## Waivers
- LEAN-WAIVER-001: Lean/Aeneas/Hax waived; Verus and TLA+ own proof split.
- PERF-WAIVER-001: No performance improvement claim; bounded execution is proven by resource caps, not benchmark speedup.
- FUZZ-COMMAND-WAIVER-001: cargo-fuzz `resource_budget -- -runs=1000` waived as invalid evidence for the current stdin-once driver; expires when a real `libfuzzer_sys::fuzz_target!` harness exists or the fuzz target is otherwise changed to consume libFuzzer inputs truthfully.

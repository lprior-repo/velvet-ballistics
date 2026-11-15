# Bounded unsafe retry repair evidence

Base: f366003714753937573e04650bf63cc4e5beb8ea. Production change is limited to `crates/vb_runtime/src/engine/action/mod.rs`; public behavior tests are `crates/vb_runtime/tests/action_resume_contract.rs`. The scoped contract is `.beads/vb-eirmk.60/unsafe-retry-contract.md`.

`red.log` records the original unsafe Writes failure issuing a next action ticket with attempt 2 and capacity 3. The red run exited 100 with 2 passing tests and 1 failing test. `green.log` records the same test target after the repair, exit 0 with 3 passing tests. `callers.log` records the final formatted source run, exit 0 with 42 passing tests and 2609 tests excluded by the explicit filter.

Final test command, copied from the repository nextest gate with bounded package, binary, filter, job, and thread selection:

```sh
flock --shared target/moon-locks/source-mutation.lock env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings" CARGO_BUILD_JOBS=2 timeout 10m rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_runtime --all-features --lib --test action_resume_contract --test-threads 4 -E 'test(engine::action::tests::) | test(resume_action) | test(retry_ticket) | binary(action_resume_contract)'
```

Source clippy ran the repository `.moon/tasks/all.yml` lint-src flags with package selection `-p vb_runtime` replacing `--workspace`, all features, source targets only, pinned nightly, `RUSTFLAGS=-Dwarnings`, and `CARGO_BUILD_JOBS=2`. It exited 0 with no diagnostics; `clippy.log` is empty. Targeted rustfmt check exited 0 with no diagnostics; `fmt.log` is empty. `git diff --check` exited 0.

No canonical taxonomy, enum tags, ticket key formula, persisted format, compiler blanket gate, source-hash receipts, or formal artifacts changed. The existing Verus resume projection does not model the newly enforced contract refusal; this patch is behavior evidence only and does not claim formal proof closure. Full `moon ci`, independent code review, and independent test review remain unperformed for this candidate. Parent retains live bead ownership and qualification status; this bounded patch does not close vb-eirmk.60.

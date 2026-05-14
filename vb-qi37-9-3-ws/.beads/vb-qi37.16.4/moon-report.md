bead_id: vb-qi37.16.4
phase: state-8
status: PASS_AFTER_REPAIR

# State 8 Machine Gate Report

Commands run from isolated workspace `/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`:

```bash
rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"
rtk cargo test -p vb_runtime --lib
moon run :quick
moon run :test
moon ci
```

## Scoped pre-gates

- `rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"` previously passed in State 7 smoke: `12 passed, 1337 filtered out`.
- `rtk cargo test -p vb_runtime --lib` previously passed in State 7 smoke: `1349 passed`.

## Machine gate failure

`moon` gates failed before State 8 can pass.

Primary failure class: `FORMAT`.

Evidence excerpt:

```text
velvet-ballastics:fmt | Diff in crates/vb_runtime/src/shard/tests.rs
... encoded_len indentation changes required ...
velvet-ballastics:fmt | Diff in crates/vb_storage/src/codec_miri_tests.rs
velvet-ballastics:fmt | Diff in crates/vb_storage/src/kani_codec.rs
velvet-ballastics:fmt | Diff in crates/vb_storage/src/lib.rs
velvet-ballastics:fmt | Diff in fuzz/fuzz_targets/decode_record.rs
velvet-ballastics:fmt | Diff in xtask/src/main.rs
velvet-ballastics:fmt | Diff in xtask/src/proof.rs
```

Additional compile failures observed after the format gate:

```text
error[E0425]: cannot find value `encoded` in this scope
   --> crates/vb_ipc/src/server/handlers.rs:243:22

error[E0063]: missing field `allows_secret_results` in initializer of `ResourceContract`
   --> crates/vb_codegen/src/proptests.rs:296:21
   --> crates/vb_codegen/src/tests.rs:902:32
   --> crates/vb_codegen/src/tests.rs:1458:24
   --> crates/vb_codegen/src/tests.rs:2106:24
   --> crates/vb_codegen/src/tests.rs:4352:24
   --> crates/vb_codegen/src/tests.rs:10152:22
   --> crates/vb_codegen/src/tests.rs:10196:20
   --> crates/vb_core/src/budget/tests.rs:1210:5
   --> crates/vb_core/src/engine/validate.rs:223:9
   --> crates/vb_core/src/workflow/tests.rs:686:9
   --> crates/vb_core/src/workflow/tests.rs:4497:13
```

Result: State 8 failed. Do not advance to QA.

---

## Final rerun after focused release repairs

See `state-8-release-gates-rerun.md`.

Command:

```bash
rtk cargo fmt -- --check && moon run :test && moon ci
```

Result:

```text
moon run :test: 9863 tests run, 9863 passed, 0 skipped
moon ci: Tasks: 19 completed (1 cached), Time: 3m 52s 48ms
```

Final status: `PASS_AFTER_REPAIR`.

---

## Post Black-Hat State 8 format repair rerun

After the State 11 Black Hat repair added real CLI `answer` IPC handling,
the current State 8 rerun initially failed at the formatter gate in scoped
file `crates/velvet_ballastics/src/main.rs`. That local `FORMAT` failure was
repaired by `holzman-rust`; see `state-8-format-repair.md`.

Current orchestrator rerun from isolated workspace
`/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`:

```bash
rtk cargo fmt -- --check && \
rtk cargo check -p velvet_ballastics -p vb_ipc --all-targets --all-features && \
moon run :test && \
moon ci
```

Result:

```text
rtk cargo fmt -- --check: PASS
rtk cargo check -p velvet_ballastics -p vb_ipc --all-targets --all-features: cargo build: 0 errors, 1 warnings (1 crates)
moon run :test: 9863 tests run: 9863 passed, 0 skipped
moon ci: Tasks: 19 completed (2 cached), Time: 2m 23s 196ms
```

Final status: `PASS_AFTER_FORMAT_REPAIR`.

---

## Post INV-002 repair State 8 rerun

After State 11 Black Hat rejected the prior implementation for bypassing
INV-002 taint enforcement, State 6 repaired the IPC protocol and handler.
Current orchestrator rerun from isolated workspace
`/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`:

```bash
rtk cargo fmt -- --check && \
rtk cargo check -p vb_ipc -p vb_runtime -p velvet_ballastics --all-targets --all-features && \
rtk cargo test -p vb_ipc --lib answer && \
rtk cargo test -p vb_runtime --lib ask_answer && \
moon run :test && \
moon ci
```

Result:

```text
rtk cargo fmt -- --check: PASS
rtk cargo check -p vb_ipc -p vb_runtime -p velvet_ballastics --all-targets --all-features: cargo build: 0 errors, 1 warnings
rtk cargo test -p vb_ipc --lib answer: 13 passed, 391 filtered out
rtk cargo test -p vb_runtime --lib ask_answer: 24 passed, 1325 filtered out
moon run :test: 9867 tests run: 9867 passed, 0 skipped
moon ci: Tasks: 19 completed (1 cached), Time: 3m 51s 31ms
```

Final status: `PASS_AFTER_INV002_REPAIR`.

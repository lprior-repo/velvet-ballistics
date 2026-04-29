# vb-65v implementation repair

## What changed

- Made `FiniteF64` non-forgeable by changing its inner `f64` field to private and keeping `FiniteF64::new` / `FiniteF64::get` as the public API.
- Replaced derived serde for `FiniteF64` with checked custom deserialization so NaN and infinities cannot enter through postcard/serde decode paths.
- Removed compiler fabrication of `BlobId`, `ListId`, and `ObjectId` from string lengths and collection counts.
- Changed unsupported string/list/object constants to return the typed `CompileError::UnsupportedConstantValue` instead of producing fake handles.
- Kept minimal scalar `save: { value: <null|bool|int> }` support because those values preserve their payload directly in `SlotValue` without arenas.
- Stopped schema default validation from calling runtime constant lowering; defaults are validated for declared schema shape without allocating or fabricating handles.
- Updated compiler tests that previously blessed fake object handles so complex/string save constants now assert rejection.
- Renamed public hot IR `CompiledNodeKind::Choose` to `CompiledNodeKind::ChooseSlot` and updated compiler/engine matching to remove the ambiguous final IR name.

## Exact command results

```text
$ rtk cargo fmt --all -- --check
(no output)
```

```text
$ rtk cargo test -p vb-core
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
Running unittests src/lib.rs (target/debug/deps/vb_core-e0d69883edcea754)
Running tests/phase1_core_types.rs (target/debug/deps/phase1_core_types-bb3783f5356e8737)
Doc-tests vb_core
cargo test: 20 passed (3 suites, 0.00s)
```

```text
$ rtk cargo test --workspace --all-targets
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.54s
Running unittests src/lib.rs (target/debug/deps/vb_compiler-30133a8a4682c922)
Running unittests src/lib.rs (target/debug/deps/vb_core-a04a67139df0078e)
Running tests/phase1_core_types.rs (target/debug/deps/phase1_core_types-9dc18691a674003b)
Running unittests src/lib.rs (target/debug/deps/vb_ipc-7746ca51f0bd386a)
Running unittests src/lib.rs (target/debug/deps/vb_storage-2741f5b0ed660898)
Running unittests src/main.rs (target/debug/deps/velvet_ballastics-8023802e85560fbb)
Running unittests src/lib.rs (target/debug/deps/fuzz_lib-2d7af1dc32081eb8)
Running unittests src/bin/binary_ipc_frame.rs (target/debug/deps/binary_ipc_frame-6acbdb25078ef348)
Running unittests src/bin/fjall_journal_append.rs (target/debug/deps/fjall_journal_append-a6a81e9935de99c3)
Running unittests src/bin/slot_value_roundtrip.rs (target/debug/deps/slot_value_roundtrip-635aae482ed08eac)
Running unittests src/bin/workflow_compile.rs (target/debug/deps/workflow_compile-b0c2a50e464c7430)
Running unittests src/bin/workflow_parse.rs (target/debug/deps/workflow_parse-080e70da98b69e0f)
Running unittests src/lib.rs (target/debug/deps/velvet_ballistics_workspace-b927780b13882de1)
Running tests/phase0_scaffold_test.rs (target/debug/deps/phase0_scaffold_test-c92b7f1ac60d2d2b)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
Running benches/velvet_ballastics.rs (target/debug/deps/velvet_ballastics-99d1311c76c6f258)
cargo test: 142 passed (15 suites, 0.09s)
```

```text
$ rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy: No issues found
```

## Remaining risks

- Real symbol/blob/list/object arenas are still not implemented in this bead; the compiler now rejects unsupported payload-bearing constants instead of pretending handles exist.
- Public YAML `save` support is intentionally narrow until cold-store/arena allocation exists: only the scalar `value` field can compile without data loss.

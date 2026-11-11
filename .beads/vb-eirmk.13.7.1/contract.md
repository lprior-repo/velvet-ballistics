# Contract: Honor configured scratch in IPC socket tests

## Scope and evidence

- Requirement: Remove hardcoded root `/tmp` filesystem paths from the audited IPC test fixtures so their socket and probe paths use Rust's configured `std::env::temp_dir()`.
- Constraint: This is test-fixture compliance only; do not change `IpcServer::bind`, public APIs, dependencies, schemas, gates, timer behavior, or files outside the four fixture files named below.
- Constraint: Audit scope is the four filesystem-path fixture files `crates/vb_ipc/src/server/impl_tests.rs`, `crates/vb_ipc/src/server/reactor_tests.rs`, `crates/vb_ipc/src/server/dispatch.rs`, and `crates/vb_ipc/src/client/tests.rs`, plus read-only behavior context in `crates/vb_ipc/src/server/impl_.rs`.
- Evidence: The supplied whole-crate inventory contains 49 hits; the IPC fixture hits are in those four fixture files, and its SHA-256 is `c98456ff1b844df596127af757e4524fa1cf2c90a4474337bf8061f55feda75a`.
- Evidence: Bead metadata pins source `c8d4044c38b558e52d857316dd43406b56c7ca3c`; this audit read the IPC files in the assigned workspace whose preserved parent is `ff9bc992` and does not alter that parent.
- Evidence: `IpcServer::bind` in `server/impl_.rs` directly binds the supplied path and maps bind errors to `IpcServerError::BindFailed`; it does not remove or truncate a pre-existing regular file.
- Evidence: The reported timer-integrated Moon run failed while writing the regular-file fixture with `EDQUOT` 122, and a later isolated reproduction passed; treat that as transient scratch exhaustion, not a reproduced timer or bind implementation defect.

## Behavior requirements

- Requirement: Every IPC filesystem path in the four fixture files derives its parent from `std::env::temp_dir()` at runtime, including server fixture helpers, reactor paths, dispatch fixtures, nonexistent client probes, the existing-directory client probe, and the nested-nonexistent-path probe.
- Requirement: Preserve existing process-ID/counter uniqueness where present, test names, cleanup behavior, typed results, error text assertions, and all scenario assertions; shorten socket leaf names only as needed to keep each encoded path below Linux `sun_path`'s 108-byte limit.
- Requirement: Keep the regular-file no-overwrite behavior unchanged: `bind_preserves_existing_regular_file` writes `b"owned contents"`, expects `IpcServerError::BindFailed`, and reads back the identical bytes after the failed bind.
- Requirement: Add a focused regression named `ipc_socket_path_uses_configured_temp_dir` that constructs the existing server fixture path without filesystem access and asserts its parent equals `std::env::temp_dir()` and its encoded path length is below 108 bytes.
- Constraint: The regression uses the inherited `TMPDIR`; it must not mutate process-global environment, access root `/tmp`, use unsafe code, or add a path framework or dependency.
- Requirement: Source review confirms all filesystem path construction in the four fixture files honors configured scratch; leave unrelated data-only `/tmp` strings outside these five audited IPC files untouched.

## Acceptance scenarios

- Given `TMPDIR=/home/lewis/.vbs`, when `ipc_socket_path_uses_configured_temp_dir` runs, then the fixture helper's parent equals `std::env::temp_dir()` and the old hardcoded helper would fail this assertion without accessing root `/tmp`.
- Given a regular file at a generated scratch path containing `b"owned contents"`, when `IpcServer::bind` is called, then it returns `IpcServerError::BindFailed` and the file remains byte-for-byte unchanged.
- Given nonexistent, empty, existing-directory, and nested-nonexistent client socket paths under configured scratch, when each existing client scenario runs, then its current `ConnectFailed` result and message assertions remain unchanged.
- Given reactor and dispatch tests run with configured scratch, when their existing socket interactions and cleanup execute, then they retain their current response, ordering, shutdown, and typed-error assertions and create sockets only below the configured temp directory.
- For every generated socket path under the acceptance `TMPDIR`, when its path bytes are measured, then the length is less than 108 bytes.

## Verification after implementation

- Focused regression command: `TMPDIR=/home/lewis/.vbs CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_ipc --all-features --lib ipc_socket_path_uses_configured_temp_dir`.
- Original no-overwrite scenario command: `TMPDIR=/home/lewis/.vbs CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_ipc --all-features --lib bind_preserves_existing_regular_file`.
- Scoped package command: `TMPDIR=/home/lewis/.vbs CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_ipc --all-features --lib`.
- Acceptance: Record each native command's actual exit and raw log; run with the safe explicit `TMPDIR`, never invoke the root-`/tmp` path or the generic Moon test wrapper without a safe inherited scratch directory.
- Acceptance: Independent CODE and separate TEST reviews inspect the same frozen source pin; keep the bead open until both reviews and scoped native evidence are recorded.

## Explicit exclusions

- Constraint: Do not run the broad timer-integrated suite as proof of an IPC defect; the reported quota failure was not reproduced in isolation.
- Constraint: Do not edit the protected `.moon` nightly-probe task; its three-line path proposal remains subject to separate user approval.
- Constraint: Do not edit production IPC code, broaden the audit beyond the five named Rust files, change any test assertion to make it pass, or claim implementation/test acceptance from this contract.

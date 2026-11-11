# Contract: Honor configured scratch in CLI and workspace filesystem tests

Bead: `vb-eirmk.13.7.2`; source pin: `c8d4044c38b558e52d857316dd43406b56c7ca3c`; `velvet-ballistics-MASTER.md` blob at the pin and its parent is `f9aea378690c44efa659d36b9c8ae630fcc2dba6`.

Scope is limited to test-owned CLI/workspace filesystem helpers and the CLI validation of a nonexistent input. Replace actual filesystem use rooted at literal `/tmp` with per-test temporary directories resolved through the configured standard temporary directory; leave IPC production/core ownership with its existing owner.

`crates/workspace_tests/tests/ipc_input_socket_preservation.rs::socket_dir` creates its fixture directory with `Builder::tempdir_in("/tmp")`; occupied socket, symlink, directory, and fresh-socket scenarios perform real binds, writes, reads, metadata checks, and fixture-owned cleanup beneath that directory.

`crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs::temp_socket_path` constructs direct `/tmp` socket paths; seven BDD tests bind/connect/write/read them, and `CleanupPath::drop` removes the socket.

`crates/velvet_ballistics/tests/support/ipc_process_crash_recovery.rs::CrashFixture::new` creates a real temporary root with `tempdir_in("/tmp")`; `CrashFixture::start_server` launches the binary and the fixture owns database, copy, and socket paths below that root.

`crates/velvet_ballistics/src/ipc_serve_tests.rs` uses literal `/tmp` socket paths in `service_turn_commits_accepted_journal_work_before_shutdown`, `service_turn_preserves_and_reports_failed_journal_staging`, and `shutdown_acknowledgement_follows_all_65_owned_queue_events_before_exit`; each binds a real Unix socket and removes its socket file. `failed_owned_shutdown_reports_wire_error_and_typed_storage_failure` already uses a tempfile-owned directory and short socket basename.

`crates/velvet_ballistics/src/ipc_restart_tests.rs::socket_submitted_scalar_ask_survives_graceful_close_and_fjall_reopen` creates its database and short-named sockets below a `Builder::tempdir_in("/tmp")` fixture.

`crates/velvet_ballistics/tests/cli_integration.rs::cli_validate_nonexistent_file_fails_with_diagnostic` passes a literal `/tmp` path to the CLI, which attempts to read the missing file. Place the still-missing filename beneath a test-owned configured-temp directory; preserve the non-success and file-diagnostic assertions without creating that input file.

Use tempfile's default temporary-directory selection (equivalent to `std::env::temp_dir()`); do not set or mutate process-global `TMPDIR`. Every temporary directory and socket must have one test-owned cleanup lifetime, preserve parallel-test uniqueness, and keep Unix socket path bytes below the platform `sun_path` limit by using short socket basenames beneath the temporary directory.

Preserve all existing behavioral assertions and typed outcomes in the listed scenarios, including socket inode/type/target/content preservation, stale/occupied refusal, listener usability, owned cleanup, IPC wire responses, queue/journal outcomes, crash-recovery lifecycle, restart/reopen behavior, and the CLI nonexistent-file diagnostic.

Keep metadata-only `/tmp` path values unchanged in `crates/workspace_tests/tests/vb_doc_boundary_inventory_integration.rs` and `crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract/validation_core.rs`; those files contain no filesystem APIs, and the paths are model inputs for workspace/evidence validation.

Keep parse-rejected argument values unchanged in `crates/velvet_ballistics/tests/cli_vb_m214_bdd_scenarios.rs` and `crates/workspace_tests/tests/cli/qa_permutation_matrix.rs`; invalid emit/durability/profile or empty socket/database arguments return parse diagnostics before filesystem access. The separate `qa_io_error_kinds_stable` fixture uses `/nonexistent-qa/nope.yaml`, not root `/tmp`, and is outside this bounded root-scratch repair.

`crates/velvet_ballistics/tests/cli_integration.rs::cli_tempdir` currently uses package `target/cli-integration-tmp`, not root `/tmp`; this contract does not expand that helper's scope.

Acceptance requires the original affected scenarios and focused test selection to run without actual reads, writes, or binds under root `/tmp`; no global environment mutation; all generated temp paths derive from the configured temporary directory; short Unix socket path and cleanup ownership retained. Run verification only after the implementation has removed the scoped root-`/tmp` operations.

No production IPC-core edits, public APIs, dependencies, schemas, gates, `.moon` settings, or unrelated test changes are in scope. No test execution, filesystem probe, or temporary-file operation under root `/tmp` is authorized by this contract.

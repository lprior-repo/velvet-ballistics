# vb-2a9i3 / vb-fnugl independent IPC review fix

Base: immutable candidate862543ed44c354846235f6676675ed86b3f19add. Separate from vb-aaqp7 candidate6a5a4d4e9861e6dc989362b496de1b6d47b4b939. Unique isolated workspace /home/lewis/src/isoloated/velvet-ballistics-vb-2a9i3-ipc-reviewfix-sol; matching Git/JJ roots, own worktree Git admin, git_target ../../../.git. No main checkout implementation or canonical moon ci was run.

Independent code review c50eb400eb44ccbbaa2900d587ec3a9125a3a470 rejected function size under the invoked black-hat25-line rule. This child extracts cohesive operations without changing public error or wire contracts. No test assertion was removed or relaxed. The existing std::io::Write test dependency is now cfg(test) after production write operations moved to their own module.

## Change and invariant mapping

- connection_io: handle_readable delegates bounded client read, dispatch and frame rejection. Request/header+token is an owned ReadyRequest value. Partial I/O delegates one operation while existing64-operation turns retain readiness when exhausted. Early four-byte magic rejection, one request/response flight, Interrupted/WouldBlock behavior and typed peer errors are preserved. Registry cloning still occurs once before client read service.
- reactor: token snapshot, client-result policy and connection construction have small helpers. Global PollFailed/ResponseEncodeFailed/ResponsePending still propagate; peer diagnostics still remove only that peer. No admission limit, readiness order or shutdown-output behavior changes.
- response_io: public send_response keeps its existing six-argument API and rejects nonempty output first; encode/reserve/append, one write and flush/reregister are separate helpers. Reservation completes before buffer mutation; written prefixes drain exactly once; would-block/interrupted output remains owned. Existing postcard/header/flush fault hooks retain the same typed external errors.
- writer/flush: one mutex still spans selection, staging, commit and retirement. PreparedBatch owns the Fjall batch and selected durability. commit_prepared returns success only after the SyncAll/empty-idempotent barrier and batch.commit; retire_committed is invoked afterward. Failed staging or commit cannot drain entries. The selected bounded FIFO/atomic groups and within-batch dedup set are preserved. Iteration over pending.iter().take(selected length) replaces the equivalent get(index) staging loop. No storage admission/shutdown API changes are included.

## Executed validation

commands.jsonl records exact argv/cwd/exit/duration and raw stdout/stderr hashes. All Cargo commands use CARGO_BUILD_JOBS=2.

- mechanical-function-size:44 scoped functions pass, maximum22 physical lines including signatures/braces; connection_io/reactor/response_io/flush files234/208/104/133 lines. Public send_response15 lines. This check covers all functions in those four small files plus send_response, not unrelated legacy helpers/tests elsewhere.
- mechanical-ipc-green:537 passed,0ignored,0filtered. The initial run failed only because moving production Write import left a test needing it; the cfg(test) import fixes compilation and raw initial failure is retained.
- mechanical-storage-queue:52 passed,0ignored,1547filtered. Includes actual commit-failure retention/retry and all-idempotent barrier regression.
- mechanical-cli-service:3 passed,0ignored,352filtered.
- mechanical-ipc-acceptance: 7 passed,0ignored,0filtered; no test was edited here.
- mechanical-source-lint: canonical source-only --lib --bins --all-features clippy bans for vb_ipc/vb_storage/velvet-ballistics passed.
- mechanical-format-check: all6 changed Rust files passed; mechanical-source-whitespace passed.

No new performance claim, new proof closure, physical disk-failure experiment or mutation campaign is claimed. Existing public send_response has six arguments for API compatibility; this lane does not silently break it to address an unreported parameter-count finding. Raw Cargo output preserves terminal blank lines. Beads stay in_progress until independent code and test review; the parent owns canonical integration/CI and closure.

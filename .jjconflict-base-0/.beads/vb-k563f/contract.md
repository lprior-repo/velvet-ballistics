# vb-k563f Rust contract

Public #[must_use] bool predicate assert_typed_ipc_error accepts exactly the 14 current vb_ipc::IpcError variants and rejects future additions. Callers must assert the returned bool; no caller may ignore the result. This provides a static oracle: any new enum variant will cause the predicate to return false, failing the assertion and alerting callers to update the predicate. Tests cover each of the 14 current variants plus a malformed-header case producing InvalidMagic{actual:0}.

The current implementation name stays assert_typed_ipc_error; future unknown variant rejection is a wildcard policy, not an executed unknown-variant test. Static production-enum/oracle drift checking remains required. No production decoder semantics change.

## Compiler fixture feature binding

Enable test-util feature on vb_compile dependency in fuzz harness to expose fixture constructors required for canonical digest property tests; no production feature defaults altered.
Canonical property build exposed the gated constructor after fixing IPC export. The fuzz package dependency enables the existing compiler test-util feature. Cargo unifies that feature for shared dependencies in a build selecting fuzz; standalone production selection does not gain a new default feature. No production API visibility edits or deleted property checks.

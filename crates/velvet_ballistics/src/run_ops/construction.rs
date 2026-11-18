use crate::app_impl::prelude::*;

pub(super) fn runtime(
    shard_count: std::num::NonZeroUsize,
    config: vb_runtime::shard::ShardConfig,
    journal: &Arc<vb_storage::FjallJournal>,
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    // Retry must not silently downgrade the durability barrier. The original per-run
    // append mode (strict vs journaled) is not recorded in the journal (admission
    // `RuntimePolicy` is `Strict` for both), so retry selects the strict barrier as the
    // safe default: it never weakens a previously-strict run and only upgrades a
    // journaled run's retry phase to a `SyncAll`-acknowledged append.
    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::clone(journal));
    vb_runtime::runtime::Runtime::new(shard_count, config, shared).map_err(|error| {
        crate::run_compiled_runtime::report_runtime_error(
            format_args!("runtime construction failed: {error}"),
            output,
        );
        CliExitCode::RuntimeFailed.into()
    })
}

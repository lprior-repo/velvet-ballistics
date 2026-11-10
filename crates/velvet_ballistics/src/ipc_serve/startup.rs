use super::*;
use crate::run_compiled_runtime::secret_presence::AvailableNames;
use vb_storage::journal::startup_discovery::StartupDiscoveryError;

const STARTUP_ENCODED_BYTE_SCAN_LIMIT: usize = 67_108_864;

pub(super) fn recover_startup(
    service: &IpcJournalService,
    runtime: &mut vb_runtime::runtime::Runtime,
    max_rows: usize,
) -> Result<AvailableNames, StartupDiscoveryError<vb_runtime::RuntimeError>> {
    let mut available_names = AvailableNames::default();
    let limit = NonZeroUsize::new(max_rows).ok_or(StartupDiscoveryError::InvalidLimit)?;
    service.journal.try_for_each_startup_run_bounded(
        limit,
        NonZeroUsize::new(STARTUP_ENCODED_BYTE_SCAN_LIMIT)
            .ok_or(StartupDiscoveryError::InvalidLimit)?,
        |run| {
            if let Ok(events) = service.journal.events_for_run_full(run)
                && let Some(admission) =
                    vb_runtime::recovery::hydrate_run_admission_from_events(&events)
            {
                available_names.install_stored(
                    runtime,
                    Arc::clone(&service.journal),
                    admission.artifact_digest(),
                );
            }
            match runtime.recover_and_resume(run) {
                Err(vb_runtime::RuntimeError::RecoveryCannotResume { reason })
                    if reason == "terminal_state" =>
                {
                    return Ok(());
                }
                result => result?,
            }
            if !runtime.tick_all()? {
                return Err(vb_runtime::RuntimeError::ShutdownInProgress);
            }
            service.flush_startup_journal()
        },
    )?;
    Ok(available_names)
}

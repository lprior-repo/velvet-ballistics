use super::{IpcJournalService, IpcServiceError, StorageWorkflowResolver};
use crate::app_impl::prelude::*;

struct IpcServeState {
    persistence: IpcJournalService,
    runtime: vb_runtime::runtime::Runtime,
    resolver: StorageWorkflowResolver,
}

pub(crate) fn cmd_ipc_serve(socket: &std::path::Path, db: &std::path::Path) -> ExitCode {
    let mut state = match initialize_service(db) {
        Ok(state) => state,
        Err(code) => return code,
    };
    let mut server = match bind_server(socket) {
        Ok(server) => server,
        Err(code) => return code,
    };
    outln!("ipc server listening on {}", socket.display());
    match serve_until_shutdown(&mut state, &mut server) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            errln!("ipc service error: {error}");
            CliExitCode::IpcError.into()
        }
    }
}

fn initialize_service(db: &std::path::Path) -> Result<IpcServeState, ExitCode> {
    let journal = open_journal(db)?;
    let queue = create_writer_queue()?;
    let runtime_journal = create_runtime_journal(&journal, &queue)?;
    let mut runtime = create_runtime(runtime_journal)?;
    let persistence = IpcJournalService { journal, queue };
    let resolver = recover_workflow_resolver(&persistence, &mut runtime)?;
    Ok(IpcServeState {
        persistence,
        runtime,
        resolver,
    })
}

fn open_journal(db: &std::path::Path) -> Result<Arc<vb_storage::FjallJournal>, ExitCode> {
    match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => Ok(Arc::new(journal)),
        Err(error) => {
            errln!("error opening journal at {}: {error}", db.display());
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn create_writer_queue() -> Result<Arc<vb_storage::JournalWriterQueue>, ExitCode> {
    match vb_storage::JournalWriterQueue::new(1024, 64, vb_storage::StorageLimits::DEFAULT) {
        Ok(queue) => Ok(Arc::new(queue)),
        Err(error) => {
            errln!("error creating journal queue: {error}");
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn create_runtime_journal(
    journal: &Arc<vb_storage::FjallJournal>,
    queue: &Arc<vb_storage::JournalWriterQueue>,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    match vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
        .shared_journal(Arc::clone(journal), Arc::clone(queue))
    {
        Ok(journal) => Ok(journal),
        Err(error) => {
            errln!("error creating runtime journal: {error}");
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn create_runtime(
    journal: vb_runtime::journal::SharedRuntimeJournal,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    let shard_count = NonZeroUsize::MIN;
    let config = vb_runtime::shard::ShardConfig::default();
    match vb_runtime::runtime::Runtime::new(shard_count, config, journal) {
        Ok(runtime) => Ok(runtime),
        Err(error) => {
            errln!("error constructing runtime: {error}");
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn recover_workflow_resolver(
    persistence: &IpcJournalService,
    runtime: &mut vb_runtime::runtime::Runtime,
) -> Result<StorageWorkflowResolver, ExitCode> {
    match super::startup::recover_startup(persistence, runtime, super::STARTUP_HEADER_SCAN_LIMIT) {
        Ok(available_names) => Ok(StorageWorkflowResolver {
            journal: Arc::clone(&persistence.journal),
            available_names,
        }),
        Err(error) => {
            errln!("error recovering startup runs: {error}");
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn bind_server(socket: &std::path::Path) -> Result<vb_ipc::server::IpcServer, ExitCode> {
    match vb_ipc::server::IpcServer::bind(socket) {
        Ok(server) => Ok(server),
        Err(error) => {
            errln!("error binding IPC socket at {}: {error}", socket.display());
            Err(CliExitCode::IpcError.into())
        }
    }
}

fn serve_until_shutdown(
    state: &mut IpcServeState,
    server: &mut vb_ipc::server::IpcServer,
) -> Result<(), IpcServiceError> {
    std::iter::from_fn(|| {
        let result = state.persistence.turn(
            server,
            &mut state.runtime,
            &mut state.resolver,
            Some(std::time::Duration::from_millis(100)),
        );
        if let Some(error) = server.take_client_error() {
            errln!("ipc peer error: {error}");
        }
        match result {
            Ok(true) => Some(Ok(())),
            Ok(false) => {
                outln!("shutdown requested");
                None
            }
            Err(error) => Some(Err(error)),
        }
    })
    .try_for_each(core::convert::identity)
}

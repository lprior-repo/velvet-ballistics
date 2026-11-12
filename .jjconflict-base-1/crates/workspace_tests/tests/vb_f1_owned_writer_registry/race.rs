use super::support::{TestResult, open, refused};
use std::sync::{Arc, Barrier, mpsc};
use vb_core::RunId;
use vb_storage::{FjallJournal, OwnedWriterError as Error, OwnedWriterLease};

type Outcome = Result<Result<OwnedWriterLease, Error>, &'static str>;
type Worker = std::thread::JoinHandle<Outcome>;

fn contender(
    journal: Arc<FjallJournal>,
    start: mpsc::Receiver<()>,
    barrier: Arc<Barrier>,
) -> std::io::Result<Worker> {
    std::thread::Builder::new()
        .name("owned-writer-contender".into())
        .spawn(move || {
            start.recv().map_err(|_| "contender start cancelled")?;
            barrier.wait();
            Ok(journal.try_acquire_owned_writer(RunId::new(77), 1))
        })
}

fn launch_pair(journal: &Arc<FjallJournal>) -> TestResult<(Worker, Worker)> {
    let barrier = Arc::new(Barrier::new(3));
    let (tx_a, rx_a) = mpsc::sync_channel(1);
    let (tx_b, rx_b) = mpsc::sync_channel(1);
    let a = contender(Arc::clone(journal), rx_a, Arc::clone(&barrier))?;
    let b = match contender(Arc::clone(journal), rx_b, Arc::clone(&barrier)) {
        Ok(worker) => worker,
        Err(error) => {
            drop(tx_a);
            let cancelled = a.join().map_err(|_| "cancelled contender unwound")?;
            assert_eq!(cancelled.err(), Some("contender start cancelled"));
            return Err(error.into());
        }
    };
    tx_a.send(())?;
    tx_b.send(())?;
    barrier.wait();
    Ok((a, b))
}

#[test]
fn two_actual_threads_have_exactly_one_owner_while_winner_is_retained() -> TestResult {
    let (_directory, journal) = open()?;
    let (first, second) = launch_pair(&journal)?;
    let a = first.join().map_err(|_| "first contender unwound")??;
    let b = second.join().map_err(|_| "second contender unwound")??;
    let (owner, loser) = match (a, b) {
        (Ok(owner), Err(error)) | (Err(error), Ok(owner)) => (owner, error),
        _ => return Err("race did not yield exactly one retained owner".into()),
    };
    assert!(
        loser == Error::Contended
            || loser
                == Error::Busy {
                    run: RunId::new(77)
                }
    );
    assert_eq!(
        refused(&journal, 77, 1)?,
        Error::Busy {
            run: RunId::new(77)
        }
    );
    journal.validate_owned_writer(&owner)?;
    assert_eq!(owner.generation(), 1);
    drop(owner);
    let fresh = journal.try_acquire_owned_writer(RunId::new(77), 1)?;
    assert_eq!(fresh.generation(), 2);
    Ok(())
}

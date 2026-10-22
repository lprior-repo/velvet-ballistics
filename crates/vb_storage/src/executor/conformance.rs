use super::receipt::hash;
use super::{ExecutorError, FjallRegister, RegisterGrant, RegisterReceipt};
use serde::{Deserialize, Serialize};
use vb_core::action_pack::{ActionPackDigest, CapabilityScopeSpec, ConformanceReportDigest};
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, StepIdx, WorkflowDigest};

/// Actual bounded adapter observations. Deserialization provides audit data only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceReport {
    /// Real initial receipt identity.
    pub first: vb_core::action_pack::ReceiptDigest,
    /// Real duplicate receipt identity.
    pub duplicate: vb_core::action_pack::ReceiptDigest,
    /// Receipt read after dropping and reopening the adapter.
    pub reopened: vb_core::action_pack::ReceiptDigest,
    /// Actual effect counter after initial write and duplicate/conflict attempts.
    pub effects_before_reopen: u64,
    /// Actual effect counter after receipt-only reconciliation.
    pub effects_after_reopen: u64,
    /// Actual target value read after reopening.
    pub target_after_reopen: i64,
    /// Named cases whose exact errors were checked during execution.
    pub checked_cases: [ConformanceCase; 4],
}

/// Exact typed rejection cases executed by the trusted catalog runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConformanceCase {
    /// Same logical ticket, another input.
    ChangedInputConflict,
    /// Same logical ticket, another target.
    ChangedResourceConflict,
    /// Same logical ticket, another action pack.
    ChangedPackConflict,
    /// Full store rejects a new ticket without evicting receipts.
    CapacityReject,
}

/// Checked origin cannot be constructed from a raw report or deserialized bytes.
#[derive(Debug, Clone)]
pub struct ExecutedConformance {
    report: ConformanceReport,
    digest: ConformanceReportDigest,
}

impl ExecutedConformance {
    /// Audit report from the actual registered adapter calls.
    pub fn report(&self) -> &ConformanceReport {
        &self.report
    }
    /// Fingerprint binds observations and exact registered implementation/suite.
    pub const fn digest(&self) -> ConformanceReportDigest {
        self.digest
    }
}

pub(super) fn execute_suite() -> Result<ExecutedConformance, ExecutorError> {
    let dir = tempfile::tempdir()?;
    let result = run_cases(dir.path());
    dir.close()?;
    let report = result?;
    let bytes = postcard::to_allocvec(&(super::catalog::version(), &report))?;
    let digest = ConformanceReportDigest::from_digest(hash(&bytes));
    Ok(ExecutedConformance { report, digest })
}

fn run_cases(path: &std::path::Path) -> Result<ConformanceReport, ExecutorError> {
    let grant = scratch_grant();
    let (first, duplicate, effects_before_reopen) = execute_initial(path, &grant)?;
    let mut reopened = FjallRegister::open(path, 2)?;
    let receipt = reopened.reconcile(&grant, grant.ticket, 42)?;
    let effects_after_reopen = reopened.effect_count()?;
    let target = reopened
        .value(grant.scope)?
        .ok_or(ExecutorError::Conformance)?;
    if receipt != first
        || duplicate != first
        || effects_before_reopen != 1
        || effects_after_reopen != 1
        || target != 42
    {
        return Err(ExecutorError::Conformance);
    }
    capacity_case(&mut reopened, &grant)?;
    Ok(ConformanceReport {
        first: first.digest(),
        duplicate: duplicate.digest(),
        reopened: receipt.digest(),
        effects_before_reopen,
        effects_after_reopen,
        target_after_reopen: target,
        checked_cases: [
            ConformanceCase::ChangedInputConflict,
            ConformanceCase::ChangedResourceConflict,
            ConformanceCase::ChangedPackConflict,
            ConformanceCase::CapacityReject,
        ],
    })
}

fn execute_initial(
    path: &std::path::Path,
    grant: &RegisterGrant,
) -> Result<(RegisterReceipt, RegisterReceipt, u64), ExecutorError> {
    let mut adapter = FjallRegister::open(path, 2)?;
    let first = adapter.execute(grant, grant.ticket, 42)?;
    let duplicate = adapter.execute(grant, grant.ticket, 42)?;
    require_conflict(adapter.execute(grant, grant.ticket, 43))?;
    let changed_scope = RegisterGrant {
        scope: CapabilityScopeSpec {
            namespace: 1,
            key: 2,
        },
        ..grant.clone()
    };
    require_conflict(adapter.execute(&changed_scope, grant.ticket, 42))?;
    let changed_pack = RegisterGrant {
        pack: ActionPackDigest::from_digest(WorkflowDigest::from_bytes([2; 32])),
        ..grant.clone()
    };
    require_conflict(adapter.execute(&changed_pack, grant.ticket, 42))?;
    Ok((first, duplicate, adapter.effect_count()?))
}

fn require_conflict(result: Result<RegisterReceipt, ExecutorError>) -> Result<(), ExecutorError> {
    match result {
        Err(ExecutorError::ReceiptConflict) => Ok(()),
        Err(other) => Err(other),
        Ok(_) => Err(ExecutorError::Conformance),
    }
}

fn capacity_case(adapter: &mut FjallRegister, grant: &RegisterGrant) -> Result<(), ExecutorError> {
    let second = another_grant(grant, 2);
    adapter.execute(&second, second.ticket, 43)?;
    let third = another_grant(grant, 3);
    match adapter.execute(&third, third.ticket, 44) {
        Err(ExecutorError::Capacity) => {}
        Err(other) => return Err(other),
        Ok(_) => return Err(ExecutorError::Conformance),
    }
    let original = adapter.reconcile(grant, grant.ticket, 42)?;
    if adapter.effect_count()? != 2 || original.effect_ordinal() != 1 {
        return Err(ExecutorError::Conformance);
    }
    Ok(())
}

fn another_grant(grant: &RegisterGrant, seq: u32) -> RegisterGrant {
    let mut ticket = grant.ticket;
    ticket.seq = SeqNo::new(u64::from(seq));
    ticket.idempotency_key =
        vb_core::action::compute_action_idempotency_key(ticket.run, ticket.run_attempt, ticket.seq, ticket.action);
    RegisterGrant {
        ticket,
        ..grant.clone()
    }
}

fn scratch_grant() -> RegisterGrant {
    let run = RunId::new(1);
    let seq = SeqNo::new(1);
    let action = ActionId::new(0);
    let ticket = ActionTicket {
        run,
        step: StepIdx::new(1),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
    };
    RegisterGrant {
        artifact: WorkflowDigest::from_bytes([1; 32]),
        pack: ActionPackDigest::from_digest(WorkflowDigest::from_bytes([1; 32])),
        ticket,
        scope: CapabilityScopeSpec {
            namespace: 1,
            key: 1,
        },
        timeout_ms: 60_000,
    }
}

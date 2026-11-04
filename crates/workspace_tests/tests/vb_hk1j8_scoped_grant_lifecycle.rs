//! Scoped capability grant lifecycle before live dispatch (vb-hk1j8).
//!
//! Strict-exact wedge: every live dispatch presents a scoped grant plus a
//! typed dispatch view. Wrong bindings, expired windows, and revocation
//! reject before any effect; recovery reads stay identity-gated only.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{ApproverId, GateDigest};
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyRule, RuntimePolicy};
use vb_core::value::SlotValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_core::{
    ActionTicket, GrantDispatchContext, GrantError, GrantId, GrantTimeMs, RevocationEpoch,
    ScopedDispatch, ScopedGrant, ScopedGrantParams,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_runtime::shard::ShardConfig;
use vb_storage::executor::{ExecutorCatalog, ExecutorError, FjallRegister, RegisterGrant};
use vb_storage::{FjallConfig, FjallJournal, JournalEvent};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(id),
        kind,
        output.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn two_action_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_hk1j8_scoped_grant_lifecycle"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                Some(2),
                Some(2),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(1),
                },
            ),
            node(
                2,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|err| err.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|err| err.to_string())
}

fn contract_for(action: ActionId) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_hk1j8_registered_action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new("velvet.register.write".into(), action)]),
    })
}

fn scope_a() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

fn issuer() -> Result<ApproverId, Box<dyn std::error::Error>> {
    ApproverId::new("grant-authority".into()).ok_or("bad issuer".into())
}

fn subject() -> Result<ApproverId, Box<dyn std::error::Error>> {
    ApproverId::new("run-operator".into()).ok_or("bad subject".into())
}

fn receipt_digest() -> GateDigest {
    GateDigest::from_bytes([0x41u8; 32])
}

fn issue_kw(
    catalog: &ExecutorCatalog,
    artifact: &vb_storage::AcceptedArtifact,
    grant: &RegisterGrant,
    ticket: ActionTicket,
    scope: CapabilityScopeSpec,
    id: u64,
    now: u64,
) -> Result<(ScopedGrant, GrantDispatchContext), Box<dyn std::error::Error>> {
    let authority = issuer()?;
    let operator = subject()?;
    let params = ScopedGrantParams {
        id: GrantId::new(id),
        issuer: authority.clone(),
        subject: operator.clone(),
        capability: Capability::new("velvet.register.write".into(), ticket.action),
        scope,
        issued_at: GrantTimeMs::new(1_000),
        expires_at: GrantTimeMs::new(2_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: RevocationEpoch::new(1),
        break_glass: false,
        approval_receipt: receipt_digest(),
        signature: None,
    };
    let scoped = catalog.issue_scoped_grant(artifact, grant, params)?;
    let dispatch = GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope,
        authority,
        operator,
        GrantTimeMs::new(now),
        RevocationEpoch::new(1),
    );
    Ok((scoped, dispatch))
}

struct Fixture {
    dir: tempfile::TempDir,
    journal: Arc<FjallJournal>,
    catalog: ExecutorCatalog,
    artifact: vb_storage::AcceptedArtifact,
    runtime: Runtime,
    run: RunId,
}

fn setup() -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(6_601);
    let workflow = two_action_workflow()?;
    let contract0 = contract_for(ActionId::new(0))?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack0 = catalog.register_pack(&contract0, scope_a(), RecoveryClass::ReconcileReceipt)?;
    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract0.clone()],
        &[pack0],
    )?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([0x11u8; 32]),
        PolicyRule {
            action: ActionId::new(0),
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        CapabilitySet::from_grants(Box::from([Capability::new(
            "velvet.register.write".into(),
            ActionId::new(0),
        )])),
        Box::from([contract_for(ActionId::new(0))?]),
    )?;
    runtime.tick_all()?;
    Ok(Fixture {
        dir,
        journal,
        catalog,
        artifact,
        runtime,
        run,
    })
}

fn pending(fixture: &Fixture, step: u16) -> TestResult<ActionTicket> {
    fixture
        .runtime
        .lookup_pending_action_ticket(fixture.run, u64::from(StepIdx::new(step).get()))
        .ok_or("expected pending action ticket".into())
}

fn decision_count(journal: &Arc<FjallJournal>, run: RunId) -> TestResult<usize> {
    Ok(vb_storage::read_run_events(journal, run)?
        .iter()
        .filter(|event| matches!(event, JournalEvent::PolicyDecisionEvent { .. }))
        .count())
}

#[test]
fn valid_grant_dispatches_narrowed_effect() -> TestResult {
    let mut fixture = setup()?;
    let ticket = pending(&fixture, 0)?;
    let grant = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket, scope_a())?;
    let (scoped, dispatch) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        40,
        1_500,
    )?;
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &dispatch, &[]),
    )?;
    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope_a())?, Some(100));
    fixture.runtime.tick_all()?;
    assert!(
        fixture
            .runtime
            .lookup_pending_action_ticket(fixture.run, 0)
            .is_none(),
        "completed action must be consumed"
    );
    assert!(pending(&fixture, 1).is_ok(), "next action must pend");
    Ok(())
}

#[test]
fn wrong_issuer_and_subject_reject_before_effect() -> TestResult {
    let mut fixture = setup()?;
    let ticket = pending(&fixture, 0)?;
    let grant = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket, scope_a())?;
    let (scoped, dispatch) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        41,
        1_500,
    )?;
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;

    let mut impostor = dispatch.clone();
    impostor.expected_issuer = ApproverId::new("impostor".into()).ok_or("bad")?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &impostor, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::IssuerMismatch
        )))
    ));

    let mut stranger = dispatch.clone();
    stranger.expected_subject = ApproverId::new("stranger".into()).ok_or("bad")?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &stranger, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::SubjectMismatch
        )))
    ));
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(decision_count(&fixture.journal, fixture.run)?, 0);
    Ok(())
}

#[test]
fn expired_and_not_yet_valid_reject() -> TestResult {
    let mut fixture = setup()?;
    let ticket = pending(&fixture, 0)?;
    let grant = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket, scope_a())?;

    let (scoped, dispatch) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        42,
        2_000,
    )?;
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &dispatch, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::Expired
        )))
    ));

    let (_, early) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        43,
        999,
    )?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &early, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::NotYetValid
        )))
    ));
    assert_eq!(adapter.effect_count()?, 0);
    Ok(())
}

#[test]
fn delegation_chain_rejected_at_dispatch() -> TestResult {
    let mut fixture = setup()?;
    let ticket = pending(&fixture, 0)?;
    let grant = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket, scope_a())?;
    let (mut scoped, dispatch) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        44,
        1_500,
    )?;
    scoped.delegated_from = Some(GrantId::new(1));
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &dispatch, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::DelegationViolation
        )))
    ));
    assert_eq!(adapter.effect_count()?, 0);
    Ok(())
}

#[test]
fn approval_cannot_broaden_scope() -> TestResult {
    let mut fixture = setup()?;
    let ticket = pending(&fixture, 0)?;
    let grant = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket, scope_a())?;
    let (scoped, mut dispatch) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant,
        ticket,
        scope_a(),
        45,
        1_500,
    )?;
    dispatch.declared_scope = CapabilityScopeSpec {
        namespace: 15,
        key: 30,
    };
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant,
        ScopedDispatch::new(&scoped, &dispatch, &[]),
    );
    assert!(matches!(
        result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::ScopeMismatch
        )))
    ));
    assert_eq!(adapter.effect_count()?, 0);
    Ok(())
}

#[test]
fn revocation_blocks_live_effect_while_history_reads() -> TestResult {
    let mut fixture = setup()?;
    let ticket0 = pending(&fixture, 0)?;
    let grant0 = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket0, scope_a())?;
    let (scoped0, dispatch0) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant0,
        ticket0,
        scope_a(),
        40,
        1_500,
    )?;
    let register_path = fixture.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let first = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        0,
        &grant0,
        ScopedDispatch::new(&scoped0, &dispatch0, &[]),
    )?;
    assert_eq!(first.value(), 100);
    fixture.runtime.tick_all()?;

    let ticket1 = pending(&fixture, 1)?;
    let grant1 = fixture
        .catalog
        .issue_register_grant(&fixture.artifact, ticket1, scope_a())?;
    let (scoped1, dispatch1) = issue_kw(
        &fixture.catalog,
        &fixture.artifact,
        &grant1,
        ticket1,
        scope_a(),
        41,
        1_500,
    )?;

    // Revoked id blocks the next live effect; nothing is journaled for it.
    let blocked = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        1,
        &grant1,
        ScopedDispatch::new(&scoped1, &dispatch1, &[GrantId::new(41)]),
    );
    assert!(matches!(
        blocked,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::Revoked
        )))
    ));
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope_a())?, Some(100));
    assert_eq!(decision_count(&fixture.journal, fixture.run)?, 1);

    // Stale epoch blocks the same way.
    let mut future = dispatch1.clone();
    future.min_epoch = RevocationEpoch::new(2);
    let stale = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        1,
        &grant1,
        ScopedDispatch::new(&scoped1, &future, &[]),
    );
    assert!(matches!(
        stale,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            GrantError::StaleEpoch
        )))
    ));
    assert_eq!(adapter.effect_count()?, 1);

    // Reconcile still reads history for the committed effect.
    let reread = fixture.runtime.reconcile_registered_action(
        &mut adapter,
        fixture.run,
        1,
        &grant1,
        &scoped1,
        &dispatch1,
    );
    assert!(
        reread.is_err(),
        "no receipt exists for the blocked effect, reconcile must report missing"
    );

    // Clean view dispatches the narrowed effect and completes the run.
    let receipt = fixture.runtime.execute_registered_action(
        &mut adapter,
        fixture.run,
        1,
        &grant1,
        ScopedDispatch::new(&scoped1, &dispatch1, &[]),
    )?;
    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 2);
    assert_eq!(adapter.value(scope_a())?, Some(100));
    fixture.runtime.tick_all()?;

    // Reopen: durable evidence survives; revocation still enforced on check.
    drop(adapter);
    drop(fixture.runtime);
    drop(fixture.journal);
    let journal = Arc::new(FjallJournal::open(
        fixture.dir.path(),
        Some(FjallConfig::default()),
    )?);
    assert!(
        !vb_storage::read_run_events(&journal, fixture.run)?.is_empty(),
        "journaled history must survive reopen"
    );
    fixture
        .catalog
        .validate_receipt(&fixture.artifact, ticket0, 100, &first)?;
    let adapter = FjallRegister::open(&fixture.dir.path().join("register"), 8)?;
    assert_eq!(adapter.effect_count()?, 2);
    assert_eq!(adapter.value(scope_a())?, Some(100));
    assert_eq!(
        scoped1.verify_for_execute(&dispatch1, &[GrantId::new(41)]),
        Err(GrantError::Revoked)
    );
    Ok(())
}

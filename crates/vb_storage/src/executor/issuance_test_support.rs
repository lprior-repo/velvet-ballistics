//! Test support for executor issuance tests.
//! Provides fixture construction, catalog, journal, and artifact helpers.

use crate::FjallJournal;
use crate::{AcceptedArtifact, admission};
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{ActionPack, CapabilityScopeSpec, RecoveryClass};
use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    Capability, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx,
    StepIdx, WorkflowDigest,
};

type BoxError = Box<dyn std::error::Error>;

pub(super) type TestResult = Result<(), BoxError>;

/// Open a fresh test journal in a temporary directory.
pub(super) fn open_test_journal() -> Result<(tempfile::TempDir, FjallJournal), BoxError> {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    Ok((dir, journal))
}

/// Build a minimal compiled workflow with no actions.
pub(super) fn minimal_workflow() -> Result<CompiledWorkflow, BoxError> {
    let mut parts = parts_from("test_minimal");
    parts.constants = Box::new([ConstValue::I64(42)]);
    parts.slot_count = 1;
    canonicalize_parts(&mut parts)?;
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

/// Build a compiled workflow containing one action invocation.
pub(super) fn action_workflow() -> Result<CompiledWorkflow, BoxError> {
    let mut parts = parts_from("test_action");
    parts.nodes = Box::new([set_node(), action_node(), finish_node(2, 1)]);
    parts.constants = Box::new([ConstValue::I64(42)]);
    parts.slot_count = 2;
    canonicalize_parts(&mut parts)?;
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn set_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }
}

fn action_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: SlotIdx::new(0),
        },
    }
}

fn finish_node(id: u16, result: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
    }
}

fn parts_from(name: &'static str) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        name: Box::<str>::from(name),
        input_slots: Box::new([]),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([set_node(), finish_node(1, 0)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn canonicalize_parts(parts: &mut WorkflowParts) -> Result<(), BoxError> {
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let hash_bytes = postcard::to_allocvec(parts)?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());
    Ok(())
}

/// Build an action contract for a register action with REQUIRED capability.
pub(super) fn action_contract() -> Result<ActionContract, BoxError> {
    let contract = ActionContract {
        id: vb_core::ActionId::new(0),
        name: ActionName::new("test_register")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new(
            "velvet.register.write".into(),
            vb_core::ActionId::new(0),
        )]),
    };
    Ok(contract)
}

/// Build a capability scope spec for register write.
pub(super) fn register_scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

/// Read the raw artifact acceptance counter from the journal.
pub(super) fn read_counter(journal: &FjallJournal) -> Result<Option<u64>, BoxError> {
    let Some(raw) = journal.artifact_acceptance.get(b"last_issued")? else {
        return Ok(None);
    };
    let arr: [u8; 8] = raw.as_ref().try_into()?;
    Ok(Some(u64::from_be_bytes(arr)))
}

/// Write a raw counter value to the journal for fault injection.
pub(super) fn write_counter(journal: &FjallJournal, value: u64) -> Result<(), BoxError> {
    journal
        .artifact_acceptance
        .insert(b"last_issued", &value.to_be_bytes())?;
    Ok(())
}

/// Verify that a persisted artifact can be read back and matches expectations.
pub(super) fn verify_artifact_readback(
    journal: &FjallJournal,
    digest: WorkflowDigest,
    expected: &AcceptedArtifact,
) -> TestResult {
    let record = journal
        .compiled_ir(digest)?
        .ok_or("artifact record not found")?;
    if record.digest != digest {
        return Err("record digest does not match expected".into());
    }
    let decoded: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    if decoded != *expected {
        return Err("decoded artifact does not match expected".into());
    }
    if admission::accepted_artifact_digest(&decoded)? != decoded.digest {
        return Err("decoded artifact digest is inconsistent".into());
    }
    if decoded.verification.digest != decoded.digest {
        return Err("verification.digest mismatch".into());
    }
    Ok(())
}

/// Complete packed-verification fixture for test modules.
pub(super) struct PackedFixture {
    pub(super) catalog: super::ExecutorCatalog,
    pub(super) workflow: CompiledWorkflow,
    pub(super) contract: ActionContract,
    pub(super) pack: ActionPack,
}

impl PackedFixture {
    /// Construct a fixture with actual conformance results.
    pub(super) fn new() -> Result<Self, BoxError> {
        let catalog = super::ExecutorCatalog::run_registered_conformance()?;
        let contract = action_contract()?;
        let pack =
            catalog.register_pack(&contract, register_scope(), RecoveryClass::ReconcileReceipt)?;
        let workflow = action_workflow()?;
        Ok(Self {
            catalog,
            workflow,
            contract,
            pack,
        })
    }

    /// Prepare an artifact from this fixture's workflow and pack.
    pub(super) fn prepare(
        &self,
        policy: RuntimePolicy,
    ) -> Result<AcceptedArtifact, super::ExecutorError> {
        self.catalog.prepare_artifact(
            &self.workflow,
            policy,
            std::slice::from_ref(&self.contract),
            std::slice::from_ref(&self.pack),
        )
    }

    /// Submit this fixture's artifact to the journal.
    pub(super) fn submit(
        &self,
        journal: &FjallJournal,
        policy: RuntimePolicy,
    ) -> Result<AcceptedArtifact, super::ExecutorError> {
        self.catalog.submit_artifact(
            journal,
            &self.workflow,
            policy,
            std::slice::from_ref(&self.contract),
            std::slice::from_ref(&self.pack),
        )
    }

    /// Check a submitted artifact against the catalog.
    pub(super) fn check(
        &self,
        journal: &FjallJournal,
        artifact: &AcceptedArtifact,
        policy: RuntimePolicy,
    ) -> TestResult {
        let restored = self.catalog.verify_artifact(artifact, policy)?;
        if &restored != &self.workflow {
            return Err("verified workflow differs from original".into());
        }
        match &artifact.verification.evidence {
            vb_core::artifact::ArtifactEvidence::Packed(packed) => {
                if packed.base.admission_policy != policy {
                    return Err("artifact admission policy differs from expected".into());
                }
            }
            other => return Err(format!("unexpected evidence: {other:?}").into()),
        }
        verify_artifact_readback(journal, artifact.digest, artifact)
    }
}

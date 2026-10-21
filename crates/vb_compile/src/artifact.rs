//! Cold compiler evidence. A checked candidate is not runtime admission authority.

use crate::{CompileErrors, check_idempotency_gates, compile_workflow_with_contracts};
use vb_core::budget::WholeWorkflowBudget;
use vb_core::{ActionContract, CompiledWorkflow, WorkflowDigest, WorkflowError};

/// The identity role of bytes bound to a checked compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationBinding {
    /// Canonical workflow parts, with the self-digest field zeroed.
    Workflow,
    /// Exact emitted postcard payload, including the workflow digest.
    Payload,
    /// Action contract declarations, in supplied order.
    ActionContracts,
    /// Resource policy encoded from the compiled workflow.
    ResourcePolicy,
}

/// An implementation check that actually executed for this candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerCheck {
    /// Structural reconstruction, resource limits and default boundedness policy.
    WorkflowReconstruction,
    /// Shared IR gates 7-15, including action-contract completeness.
    SharedIrGatesWithContracts,
    /// Static retry/idempotency contract declarations; no executor guarantee.
    IdempotencyDeclarations,
    /// Registry IDs and names satisfy numeric runtime dispatch.
    ActionRegistry,
    /// Recomputed canonical workflow digest equals the claimed workflow digest.
    CanonicalWorkflowDigest,
    /// Whole-workflow budget computed from the checked IR.
    WholeWorkflowBudget,
}

/// Source provenance is absent for callers supplying only compiled IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProvenance {
    /// No source-spec bytes were provided or checked.
    CompiledIrOnly,
    /// The canonical compiler ran on these exact source-spec bytes.
    CompiledSourceSpec(WorkflowDigest),
}

/// A failure cannot produce a checked compiler candidate.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactVerificationError {
    /// Canonical source compilation failed.
    #[error("source compilation failed: {0}")]
    Compilation(#[from] CompileErrors),
    /// Structural reconstruction or budget computation failed.
    #[error("compiled workflow validation failed: {0}")]
    Workflow(#[from] WorkflowError),
    /// A shared IR gate failed.
    #[error("compiled IR gate failed: {0}")]
    Validation(#[from] vb_validate::ValidationError),
    /// Registry cannot be consumed by numeric runtime dispatch.
    #[error("action registry invalid: {0}")]
    ActionRegistry(#[from] vb_core::action::ActionRegistryError),
    /// Canonical encoding failed.
    #[error("artifact evidence encoding failed: {0}")]
    Encoding(#[from] postcard::Error),
    /// Bytes do not match the identity recorded for that binding role.
    #[error("{binding:?} digest mismatch: claimed {claimed:?}, computed {computed:?}")]
    DigestMismatch {
        /// Which semantic identity disagrees.
        binding: CompilationBinding,
        /// The candidate's expected digest.
        claimed: WorkflowDigest,
        /// Digest recomputed from the supplied bytes.
        computed: WorkflowDigest,
    },
}

/// Immutable evidence from executed compiler checks, bound to emitted bytes.
///
/// This type cannot be deserialized or constructed from claims. It contains no
/// durability, replay-conformance or external-effect-safety assertion. Runtime
/// consumers must still independently verify the candidate before admission.
#[derive(Debug, Clone)]
pub struct VerifiedCompilation {
    workflow: CompiledWorkflow,
    payload: Box<[u8]>,
    contracts: Box<[ActionContract]>,
    source: SourceProvenance,
    payload_digest: WorkflowDigest,
    contract_digest: WorkflowDigest,
    policy_digest: WorkflowDigest,
    budget: WholeWorkflowBudget,
}

impl VerifiedCompilation {
    /// Checked immutable workflow.
    #[must_use]
    pub const fn workflow(&self) -> &CompiledWorkflow {
        &self.workflow
    }

    /// Exact serialized workflow parts bound by the payload digest.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Contract declarations checked for this workflow.
    #[must_use]
    pub fn contracts(&self) -> &[ActionContract] {
        &self.contracts
    }

    /// Whether canonical source compilation was performed.
    #[must_use]
    pub const fn source_provenance(&self) -> SourceProvenance {
        self.source
    }

    /// Hash of the exact emitted IR payload.
    #[must_use]
    pub const fn payload_digest(&self) -> WorkflowDigest {
        self.payload_digest
    }

    /// Hash of the encoded action contract declarations.
    #[must_use]
    pub const fn action_contract_digest(&self) -> WorkflowDigest {
        self.contract_digest
    }

    /// Hash of the encoded resource contract.
    #[must_use]
    pub const fn policy_digest(&self) -> WorkflowDigest {
        self.policy_digest
    }

    /// Whole-workflow budget derived from the checked IR.
    #[must_use]
    pub const fn budget(&self) -> &WholeWorkflowBudget {
        &self.budget
    }

    /// Exactly the checks executed by the compiled-artifact verifier.
    #[must_use]
    pub const fn checks(&self) -> [CompilerCheck; 6] {
        [
            CompilerCheck::WorkflowReconstruction,
            CompilerCheck::SharedIrGatesWithContracts,
            CompilerCheck::IdempotencyDeclarations,
            CompilerCheck::ActionRegistry,
            CompilerCheck::CanonicalWorkflowDigest,
            CompilerCheck::WholeWorkflowBudget,
        ]
    }
}

/// Compile exact source bytes and retain their identity separately from IR.
pub fn compile_verified_artifact(
    source: &[u8],
    contracts: &[ActionContract],
) -> Result<VerifiedCompilation, ArtifactVerificationError> {
    let workflow = compile_workflow_with_contracts(source, contracts)?;
    let mut candidate = verify_compiled_artifact(&workflow, contracts)?;
    candidate.source = SourceProvenance::CompiledSourceSpec(digest(source));
    Ok(candidate)
}

/// Recheck compiled IR and contracts without inventing source provenance.
pub fn verify_compiled_artifact(
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> Result<VerifiedCompilation, ArtifactVerificationError> {
    let mut parts = workflow.to_parts();
    let workflow = CompiledWorkflow::try_from_parts(parts.clone())?;
    vb_validate::shared::validate_with_contracts(&parts, contracts)?;
    vb_core::action::validate_action_registry(contracts)?;
    check_idempotency_gates(contracts)?;
    let payload = postcard::to_allocvec(&parts)?.into_boxed_slice();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    check_digest(
        CompilationBinding::Workflow,
        workflow.digest(),
        &postcard::to_allocvec(&parts)?,
    )?;
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    Ok(VerifiedCompilation {
        workflow,
        payload_digest: digest(&payload),
        payload,
        contracts: contracts.to_vec().into_boxed_slice(),
        source: SourceProvenance::CompiledIrOnly,
        contract_digest: digest(&postcard::to_allocvec(contracts)?),
        policy_digest: digest(&postcard::to_allocvec(&parts.resource_contract)?),
        budget,
    })
}

fn digest(bytes: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(bytes).into())
}

fn check_digest(
    binding: CompilationBinding,
    claimed: WorkflowDigest,
    bytes: &[u8],
) -> Result<(), ArtifactVerificationError> {
    let computed = digest(bytes);
    if claimed != computed {
        return Err(ArtifactVerificationError::DigestMismatch {
            binding,
            claimed,
            computed,
        });
    }
    Ok(())
}

use super::receipt::hash;
use super::{ExecutedConformance, ExecutorError, RegisterGrant};
use vb_core::action_pack::{
    ActionPack, ActionPackDigest, CapabilityScopeSpec, ExecutorIdentity, ExecutorVersionDigest,
    IdempotencyClass, ManifestDigest, ReceiptSchemaDigest, RecoveryClass, TimeoutPolicy,
};
use vb_core::{ActionContract, ActionTicket, Idempotency, RetrySafety, SideEffect};

/// Trusted built-in selector. Construction always executes the concrete scratch suite.
/// Cloning preserves that checked origin; raw report deserialization cannot create it.
#[derive(Debug, Clone)]
pub struct ExecutorCatalog {
    checked: ExecutedConformance,
}

impl ExecutorCatalog {
    /// Execute duplicate/conflict/reopen/capacity cases using isolated scratch state.
    pub fn run_registered_conformance() -> Result<Self, ExecutorError> {
        Ok(Self {
            checked: super::conformance::execute_suite()?,
        })
    }

    /// Inspect actual observations; they are not provider or remote-service guarantees.
    pub fn conformance(&self) -> &ExecutedConformance {
        &self.checked
    }

    /// Build a checked local-register declaration. Recovery remains an explicit axis.
    pub fn register_pack(
        &self,
        contract: &ActionContract,
        scope: CapabilityScopeSpec,
        recovery: RecoveryClass,
    ) -> Result<ActionPack, ExecutorError> {
        let pack = ActionPack {
            action: contract.id,
            manifest: ManifestDigest::from_digest(hash(&postcard::to_allocvec(contract)?)),
            executor: ExecutorIdentity::FjallRegisterV1,
            executor_version: version(),
            scope,
            receipt_schema: receipt_schema(),
            timeout: TimeoutPolicy {
                timeout_ms: contract.timeout_ms,
            },
            idempotency: IdempotencyClass::KeyEnforcedByExecutor,
            recovery,
            duplicate_ticket_test: self.checked.digest(),
        };
        self.check_pack(contract, &pack)?;
        Ok(pack)
    }

    pub(super) fn check_pack(
        &self,
        contract: &ActionContract,
        pack: &ActionPack,
    ) -> Result<(), ExecutorError> {
        check_contract(contract)?;
        if pack.action != contract.id
            || pack.executor != ExecutorIdentity::FjallRegisterV1
            || pack.executor_version != version()
            || pack.receipt_schema != receipt_schema()
            || pack.manifest != ManifestDigest::from_digest(hash(&postcard::to_allocvec(contract)?))
            || pack.duplicate_ticket_test != self.checked.digest()
            || pack.timeout.timeout_ms != contract.timeout_ms
            || pack.idempotency != IdempotencyClass::KeyEnforcedByExecutor
            || !matches!(
                pack.recovery,
                RecoveryClass::Blocked | RecoveryClass::ReconcileReceipt
            )
        {
            return Err(ExecutorError::PackMismatch);
        }
        Ok(())
    }

    /// Trusted local operator authorizes one exact artifact/ticket/resource.
    /// Runtime additionally requires that the ticket and input come from its live pending action.
    pub fn issue_register_grant(
        &self,
        artifact: &crate::AcceptedArtifact,
        ticket: ActionTicket,
        scope: CapabilityScopeSpec,
    ) -> Result<RegisterGrant, ExecutorError> {
        self.verify_artifact(artifact, vb_core::RuntimePolicy::Strict)?;
        let pack = self.pack_for(artifact, ticket.action)?;
        if pack.scope != scope {
            return Err(ExecutorError::Unauthorized);
        }
        let grant = RegisterGrant {
            artifact: artifact.digest,
            pack: pack_digest(pack)?,
            ticket,
            scope,
            timeout_ms: pack.timeout.timeout_ms,
        };
        grant.validate(ticket)?;
        Ok(grant)
    }
    /// Recheck an opaque adapter receipt against the exact artifact and logical invocation.
    pub fn validate_receipt(
        &self,
        artifact: &crate::AcceptedArtifact,
        ticket: ActionTicket,
        value: i64,
        receipt: &super::RegisterReceipt,
    ) -> Result<(), ExecutorError> {
        let pack = self.pack_for(artifact, ticket.action)?;
        let grant = self.issue_register_grant(artifact, ticket, pack.scope)?;
        let expected = super::receipt::record(&grant, value, receipt.effect_ordinal())?;
        super::receipt::validate_record(&receipt.0)?;
        if receipt.0 != expected {
            return Err(ExecutorError::ReceiptConflict);
        }
        Ok(())
    }

    /// Recovery authority is explicit and independent from idempotency enforcement.
    pub fn require_receipt_recovery(
        &self,
        artifact: &crate::AcceptedArtifact,
        action: vb_core::ActionId,
    ) -> Result<(), ExecutorError> {
        self.verify_artifact(artifact, vb_core::RuntimePolicy::Strict)?;
        if self.pack_for(artifact, action)?.recovery != RecoveryClass::ReconcileReceipt {
            return Err(ExecutorError::PackMismatch);
        }
        Ok(())
    }
}

fn check_contract(contract: &ActionContract) -> Result<(), ExecutorError> {
    let cap = vb_core::Capability::new("velvet.register.write".into(), contract.id);
    if contract.side_effect != SideEffect::Writes
        || contract.idempotency != Idempotency::IdempotentExternal
        || contract.retry_safety != RetrySafety::KeyRequired
        || contract.input_slot_count != 1
        || contract.output_slot_count != 1
        || contract.max_input_bytes < 11
        || contract.max_output_bytes < 11
        || contract.timeout_ms == 0
        || contract.timeout_ms > 60_000
        || contract.required_capabilities.as_ref() != [cap]
    {
        return Err(ExecutorError::PackMismatch);
    }
    Ok(())
}

pub(super) fn pack_digest(pack: &ActionPack) -> Result<ActionPackDigest, ExecutorError> {
    Ok(ActionPackDigest::from_digest(hash(&postcard::to_allocvec(
        pack,
    )?)))
}

pub(super) fn receipt_schema() -> ReceiptSchemaDigest {
    ReceiptSchemaDigest::from_digest(hash(include_bytes!("receipt.rs")))
}

pub(super) fn version() -> ExecutorVersionDigest {
    let mut hasher = blake3::Hasher::new();
    [
        include_bytes!("register.rs").as_slice(),
        include_bytes!("receipt.rs").as_slice(),
        include_bytes!("authority.rs").as_slice(),
        include_bytes!("conformance.rs").as_slice(),
        include_bytes!("catalog.rs").as_slice(),
    ]
    .iter()
    .for_each(|source| {
        hasher.update(source);
    });
    ExecutorVersionDigest::from_digest(vb_core::WorkflowDigest::from_bytes(
        hasher.finalize().into(),
    ))
}

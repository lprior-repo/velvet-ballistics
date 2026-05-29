use vstd::prelude::*;

verus! {
pub enum CrateBoundary { VbCli, VbStorageDiagnostic, VbCore, VbRuntime, VbIpc }
pub open spec fn doctor_allowed(boundary: CrateBoundary) -> bool {
    boundary is VbCli || boundary is VbStorageDiagnostic
}
pub proof fn lemma_runtime_core_forbidden(boundary: CrateBoundary)
    requires boundary is VbCore || boundary is VbRuntime || boundary is VbIpc
    ensures !doctor_allowed(boundary)
{}
}

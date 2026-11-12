---
section: 9
title: "Rust SDK Public Surface"
parent: velvet-ballistics-MASTER.md
---

## 9. Rust SDK Public Surface

The SDK has four surfaces:

```text
Compiler SDK
Runtime SDK
Action SDK
Testkit SDK
```

### Compiler SDK

```rust
pub struct Compiler;

impl Compiler {
    pub fn verify(&self, definition: WorkflowDefinition) -> Result<VerificationOutcome>;
    pub fn compile(&self, accepted: AcceptedWorkflow) -> Result<AcceptedArtifact>;
    pub fn explain(&self, definition: WorkflowDefinition) -> Result<WorkflowExplanation>;
    pub fn graph(&self, definition: WorkflowDefinition) -> Result<WorkflowGraph>;
}

pub enum VerificationOutcome {
    Accepted(AcceptedWorkflow),
    Rejected(RejectedWorkflow),
}
```

No arbitrary `VerificationReport` may be compiled. Only `AcceptedWorkflow` can become `AcceptedArtifact`.

### Runtime SDK

```rust
pub struct RuntimeClient;

impl RuntimeClient {
    pub fn install_artifact(&self, artifact: &AcceptedArtifact) -> Result<InstalledArtifact>;

    pub fn submit_artifact<I>(
        &self,
        artifact: ArtifactDigest,
        input: I,
        options: SubmitOptions,
    ) -> Result<SubmitReceipt>
    where
        I: EncodeInput;

    pub fn inspect(&self, run: RunId) -> Result<RunInspection>;
    pub fn events(&self, run: RunId, page: EventPageRequest) -> Result<EventPage>;
    pub fn replay(&self, run: RunId, options: ReplayOptions) -> Result<ReplayReport>;
    pub fn incident(&self, run: RunId, options: IncidentOptions) -> Result<IncidentReport>;
    pub fn cancel(&self, run: RunId, options: CancelOptions) -> Result<CancelReceipt>;
    pub fn answer(&self, run: RunId, ask: AskId, payload: AnswerPayload) -> Result<AnswerReceipt>;
}
```

All observation APIs are bounded by cursor, page size, or explicit limit. Mutating APIs return durable receipts, not `()`.

### Action SDK

```rust
pub struct ActionManifest;
pub trait ActionExecutor;
```

Manifest and executor are separate. The compiler consumes manifests. The runtime dispatches executors by numeric `ActionId`.

### Testkit SDK

```rust
pub struct TestRuntime;

impl TestRuntime {
    pub fn with_production_engine() -> Result<Self>;
    pub fn compile_and_install(&mut self, definition: WorkflowDefinition) -> Result<InstalledArtifact>;
    pub fn mock_action(&mut self, action: ActionId, mock: MockAction) -> Result<()>;
    pub fn submit<I: EncodeInput>(&mut self, artifact: ArtifactDigest, input: I) -> Result<SubmitReceipt>;
    pub fn crash_at(&mut self, point: CrashPoint) -> Result<()>;
    pub fn recover(&mut self) -> Result<RecoveryReport>;
    pub fn replay(&self, run: RunId) -> Result<ReplayReport>;
}
```

The testkit must use the same compiler, artifact, runtime, storage, and replay code as production.

---


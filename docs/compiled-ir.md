# Compiled IR

Compiled IR is the numeric runtime contract between the cold YAML compiler and the engine. The authoritative types are in `crates/vb_core/src/workflow/mod.rs`; the master build contract defines acceptance.

## Artifact and input compatibility

The compiled artifact IR version is `2.0.0`. Recursive input constraints, portable default trees, retained source symbol payloads, and per-action retry policy change the Postcard layout. Regenerate earlier compiled artifacts from YAML; do not reinterpret old bytes as current IR. The retained owned-input record uses wire version 2 and binds the inputs to the complete compiled input declarations and source metadata.

`CompiledWorkflow` retains its name, raw and semantic digests, numeric nodes, expression and accessor programs, constants, source metadata, declared input slots, resource contract, and derived loop ownership. Source symbols are installed as an immutable arena prefix. Owned input admission interns equal text payloads to one symbol identity, including when the workflow has no source symbols, and relocates caller symbols around any source prefix before execution. Container aliases and per-value taints remain intact. Recovery validates the same prefix and canonical roots; retained records with noncanonical symbol identities fail closed. Symbol counts alone do not establish payload ownership.

Construction checks that the actual source prefix fits the arena contract. Owned admission checks actual occupancy after relocation and default materialization. Execution retains checked arena insertion and returns a typed error at capacity. Admission does not reserve every possible future allocation across loops and branches.

## YAML lowering

The canonical compile path accepts nested multi-step `choose`, `together`, `for_each`, `collect`, `reduce`, and `repeat` bodies. Named outputs and temporary slots remain lexically scoped. Every emitted node and private temporary contributes to checked layout allocation. Loop source values and taints remain available across iterations and scheduler slices.

A Repeat body or Together branch selects its last value-producing authored step in that same lexical scope. Wait and Choose do not replace an earlier result, and branch-local values do not escape Choose. The selected value must remain definitely assigned on every successful exit; a skipped conditional writer cannot manufacture a result. These bodies materialize Null only when they have no lexical value-producing step; the loop attempt counter is never a result. Reduce retains its numeric-result requirement. Body exit materialization is included in checked IR layout and loop budgets.

ForEach publishes a list containing each iteration's last executed value-producing step, in input order. An empty input produces an empty list. An iteration in which no producer executes contributes Null; Null values and nested lists remain actual elements. The private per-iteration result resets before the body runs. Named conditional outputs still require definite assignment and receive no default when skipped. Named step results, lexical list variables, and list accessors can supply the input. The compiler uses a proven source bound or the runtime list-size policy limit, and counts result capture and append nodes in the checked layout and loop budget. Intermediate lists consume checked arena capacity; exhaustion produces a typed error.

`at_once` is a positive upper bound, defaulting to 1. The synchronous runtime processes one item at a time, including when a larger cap is supplied. A suspended action or Ask completes before that iteration is captured and the next item starts. This scheduling behavior provides no parallel-throughput guarantee.

ForEach bodies accept dynamic references in equality and helper arguments. Nested numeric and Boolean operators still require statically known operand types. Known helper argument mismatches remain compile errors; dynamic helper argument mismatches produce typed runtime failures.

Idle ticks resume work yielded at a pending step, including one-step scheduler slices. Pending action, Ask, and Wait boundaries retain their issued authority until the matching host response. Nested Ask answers bind to named step outputs, preserve answer taint, and complete the suspended step before loop rearming.

Together preserves ordered branch values, including Null and nested lists, with their per-item taints. An error route releases reservations for scopes it exits and retains reservations for enclosing scopes containing the handler. Cold budgets include error-handler paths.

Expression stacks retain each operand's taint alongside its value. List append preserves existing element taints and records the appended operand's taint. Iteration binds each selected item with the join of its stored taint and the source container's taint; a prior item's taint does not become the next item's container taint. Execution and deterministic replay share these payload rules.

A successful owned loop backedge can rearm a failed body step only when its declared forward error-handler chain completed inside that owner. The runtime validates the entire region before resetting states; ordinary Failed-to-Pending transitions remain forbidden. A nested scope abandoned by recovery needs an actual failed error edge crossing its boundary and a completed enclosing handler. Handler exits contribute to the enclosing budget rather than the abandoned child's iteration multiplier. Legacy numeric ErrorHandler wrappers inside owned loops remain unsupported.

YAML `secrets` declarations retain alias and requirement names and bind `$secrets.alias` to an immutable Secret-tainted opaque reference. CLI `run`, `run-compiled`, cold submission, `bench-run`, `retry`, and pristine `resume` check whether each declared environment key exists, including an empty value; invalid host keys are unavailable. IPC workflow resolution prepares declared secret availability before runtime admission. Embedding hosts install confirmed requirement names with `Runtime::set_available_secret_names`. Availability does not grant action capabilities, and artifact and capability authorization precede presence checks. Missing requirements reject admission before a run or its journal records are created. Raw credential values are absent from compiled metadata and runtime secret roots. Cold journal hydration restores immutable reference defaults only after validating recovery authority; raw recovery commands must already carry each exact Secret-tainted root.

Step `if`, `then`, and `on_error` apply to the supported primitives. Error continuations use the failed instruction’s actual write semantics. Failure-atomic starts retain their prior slot facts; instructions that can fail after writing retain every possible partial-write fact. A configured numeric error slot records the failed step index with Clean taint; deterministic execution emits its actual value and taint before StepFailed. `then` and handler targets are validated lexical forward references. Actions accept exactly one of `input` or `with`; `with` supports owned scalar and composite values and references. Variables and top-level result mappings retain composite payloads. Use either an explicit `finish` step or a top-level `result` mapping.

`try_again` applies to action steps and retains a finite attempt bound and fixed delay in compiled numeric metadata. Runtime action contracts still govern retry safety and idempotency. No YAML or duration text is interpreted by the runtime. A delayed failure is persisted atomically with a versioned timer containing its exact failed ticket, UTC deadline, generation, and digest-bound full checkpoint. Restart restores that authority and execution mode, rejects early scheduling and forged tickets, and preserves a retry already rescheduled before shutdown. Legacy waits without that authority cannot recover a delayed retry.

Inline examples contain an optional input mapping and a required expected value. CLI verification admits their inputs through the owned input schema boundary, executes deterministic steps across bounded scheduler slices, and compares the actual scalar or composite result. External actions and host suspension require their own execution environment; inline examples do not perform external effects.

The older `parse_ast` facade retains its narrower legacy AST contract. Use the canonical compiler for the YAML surface above. Both public facades enforce the caller's byte, scalar, nesting, node, sequence, and mapping limits.

## Input admission

Input declarations retain optionality, nullability, secrecy, numeric and length bounds, list element kinds, object fields, closed-object policy, and scalar or owned defaults. Nested schemas and default trees are bounded during binary decoding and validated before workflow construction. Unsupported pattern constraints fail explicitly.

Owned admission validates actual payload graphs, applies nested defaults, preserves secret taint, and rejects unknown fields and invalid handles. Scalar admission accepts only self-contained values or symbols backed by compiled source metadata. Text, lists, objects, and their defaults require owned input payloads; scalar IPC cannot supply their arenas. A rejected submission cannot create a run or append its admission journal events.

Recovery reads scalar and owned retained records under the validated admission receipt. Scalar roots recover secrecy from compiled declarations; owned roots restore their exact canonical arena checkpoint without relocation or default repair. Initial payloads do not reconstruct later runtime allocations: journal-only recovery rejects missing runtime arena state, while full retry checkpoints remain authoritative.

Bound attempt-1 scalar Repeat traces with no declared inputs can recover across untimed Ask boundaries through chronological replay. Recovery checks control edges, current-step value and taint captures, success and failure receipts, paired answers, and the terminal result against the compiled workflow. It restores the replayed frame, including its execution count. Ask activation shares the live operand validator; the low-level deterministic replay API retains its suspension barrier. Incomplete captures and captures attributed to failed steps reject recovery.

Defaulted retained inputs must be present in the canonical mapping. Owned defaults and taints converge within the shared traversal bound before hashing, including aliases across roots. Restore requires exact roots and checkpoint equality; revalidation cannot silently repair retained data. Cold schema validation rejects keywords incompatible with their declared kind.

## Constructor validation

`WorkflowParts` is untrusted input. `CompiledWorkflow::try_from_parts` checks numeric bounds, required outputs, reference tables, loop topology, finite resource budgets, input schemas, retry policies, and edge-sensitive slot flow before execution. The public validation gates share the same slot analysis.

Slot flow checks definite assignment and possible operand kinds through branches, zero-iteration exits, ordered parallel branches, loop feedback, and error continuations. Required or defaulted inputs seed assignment; optional inputs do not. Successful Ask continuation supplies only the answer authorized by its immediate `AskResume`; bypass and failure edges cannot manufacture an answer. Dynamic action results and accessor leaves retain runtime type checks.

Analysis scratch space and convergence work are bounded. Capacity exhaustion is a typed validation failure, never permission to skip analysis. Derived loop ownership is recomputed from validated IR rather than trusted from serialized caches.

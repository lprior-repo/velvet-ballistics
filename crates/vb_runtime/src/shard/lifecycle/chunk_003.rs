// Action completion and failure preflight helpers. Implementation is split
// across focused chunks under `chunk_003_chunks/`. All chunks share this
// module's `use` declarations and are `include!`-d into the lifecycle shell
// to keep the public surface and tests unchanged. Splitting by domain
// responsibility:
//
// - `chunk_001_preflight_completion` - `use` declarations, completion
//   preflight (`preflight_action_completion`) plus every size, taint, and
//   contract-validation rejector.
// - `chunk_002_failure_preflight` - failure preflight / apply split
//   (`ActionFailurePreflight`, `preflight_action_failure`,
//   `apply_action_failure_preflight`) and `write_failure_slot`.
//
// ## Live preflight vs. static preflight
//
// These preflight functions run **during live execution** inside the runtime
// engine. They validate action completion tickets, output sizes, taint
// constraints, and contracts at the moment an action finishes. This is
// fundamentally different from the **static preflight** provided by the
// `vb_cli::commands_workflow::simulate` module, which walks the compiled IR
// without executing anything.
//
// |                    | Live preflight (this module) | Static preflight (vb_cli simulate) |
// |--------------------|---|---|
// | **When**           | During execution, after action dispatch | Before any run starts |
// | **Input**          | ActionTicket + ActionOutputReady | CompiledWorkflow (IR) |
// | **Mutates state?** | Yes — writes slots, advances PC | No — read-only analysis |
// | **Accesses storage** | Yes — journals events | No |
// | **Purpose**        | Safety gate at action boundary | Dry-run structural analysis |

include!("chunk_003_chunks/chunk_001_preflight_completion.rs");
include!("chunk_003_chunks/chunk_002_failure_preflight.rs");
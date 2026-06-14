#![forbid(unsafe_code)]
//! CLI constants and static configuration.

use std::process::ExitCode;

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const HELP: &str = "\
velvet-ballistics - compiled workflow runtime

commands:
  validate   <workflow.yaml> [--emit text|yaml|postcard]          Validate a workflow definition
  verify     <workflow.yaml> [--profile <quick|standard|full>] [--emit text|yaml|postcard]  Verify a workflow
  explain    <workflow.yaml> [--emit text|yaml|postcard]          Explain dry-run execution plan
  compile    <workflow.yaml> --emit <ir|yaml|postcard> --out <file>  Compile a workflow
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>] [--emit text|yaml|postcard]
             [--step <id> --step-input <file>]                                 Run a single step in isolation
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>] [--emit text|yaml|postcard]
  ipc-serve  --socket <path> --db <path>               Start IPC server
  inspect    <run_id> --db <path> [--emit text|yaml|postcard]     Inspect a run
   events     <run_id> --db <path> [--status <status>] [--limit <N>] [--emit text|yaml|postcard]     List run events
  replay     <run_id> --db <path> [--emit text|yaml|postcard]     Replay a run from journal
  trace      <run_id> --db <path> [--step <N>] [--action <N>] [--status <status>]
             [--since-seq <N>] [--until-seq <N>] [--limit <N>] [--emit text|yaml|postcard]
                                                        Show step-by-step execution trace
  retry      <run_id> --db <path> [--step <N>] [--emit text|yaml|postcard]  Retry a failed run from last successful step
  resume     <run_id> --db <path> [--emit text|yaml|postcard]     Resume a suspended run
  cancel     <run_id> --db <path> [--reason <text>] [--emit text|yaml|postcard]  Cancel a run
  bench-run  <workflow.yaml> [--emit text|yaml|postcard]          Benchmark a workflow
  doctor     [--db <path>] [--emit text|yaml|postcard]            Run diagnostic checks
  answer     <run_id> --slot <N> --value <postcard SlotValue file> --db <path> [--emit text|yaml|postcard]  Answer a suspended step
  graph      <workflow.yaml> [--emit text|yaml|postcard]          Output control flow graph in DOT format
  diff       <workflow.yaml> --against <old.yaml> [--emit text|yaml|postcard]  Compare workflow semantics
  diff       <run_a> <run_b> --db <path> [--emit text|yaml|postcard]  Compare two runs
  incident   <run_id> --db <path> [--emit text|yaml|postcard]     Black-box failure report
  submit     <workflow.yaml> --input-bin <file> --db <path> --durability <mode> [--emit text|yaml|postcard]  Submit workflow run
  simulate   <workflow.yaml> [--emit text|yaml|postcard]     Dry-run workflow without executing actions
  ai-context <run_id> --db <path> [--emit text|yaml|postcard]  Emit compact AI context packet for a run
  help                                                Print this message
  version                                             Print version
  agent-context [--deliver stdout|file:<absolute-path>|webhook:<url>]  Emit or deliver versioned AI-agent CLI schema
  status     [--active-runs <N>] [--queue-depth <N>] [--trace-dropped <N>] [--db <path>] [--emit text|yaml]  Report runtime shard status (with live Fjall probe when --db is supplied)
  system status [--profile <quick|standard|full>] [--server none] [--db <path>] [--emit text|yaml]  Report bounded system health (probes Fjall when --db is supplied)
  action list [--emit text|yaml|postcard]                       List registered action contracts
  action inspect <action-name> [--emit text|yaml|postcard]       Show one registered action contract

options:
  --emit text      Output human-readable text (default)
  --emit yaml      Output structured YAML-compatible text
  --emit postcard  Output binary machine payload where supported
  --deliver   Deliver supported artifacts to stdout, file:<absolute-path>, or webhook:<url> (webhook currently returns structured refusal)

architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str = "INPUT_MAPPING_FAILED: input-bin decode failed";
const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

#![forbid(unsafe_code)]

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const HELP: &str = "\
velvet-ballistics - compiled workflow runtime

commands:
  validate   <workflow.yaml> [--emit text|yaml|postcard]
  verify     <workflow.yaml> [--profile <quick|standard|full>] [--emit text|yaml|postcard]
  explain    <workflow.yaml> [--emit text|yaml|postcard]
  compile    <workflow.yaml> --emit <ir|yaml|postcard> --out <file>
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>] [--emit text|yaml|postcard]
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>] [--emit text|yaml|postcard]
  ipc-serve  --socket <path> --db <path>
  inspect    <run_id> --db <path> [--emit text|yaml|postcard]
  events     <run_id> --db <path> [--emit text|yaml|postcard]
  replay     <run_id> --db <path> [--emit text|yaml|postcard]
  trace      <run_id> --db <path> [--emit text|yaml|postcard]
  retry      <run_id> --db <path> [--emit text|yaml|postcard]
  resume     <run_id> --db <path> [--emit text|yaml|postcard]
  cancel     <run_id> --db <path> [--reason <text>] [--emit text|yaml|postcard]
  action     <list|inspect> [--registry <registered|empty|uninitialized>] [--emit text|yaml|postcard]
  bench-run  <workflow.yaml> [--emit text|yaml|postcard]
  doctor     [--db <path>] [--emit text|yaml|postcard]
  answer     <run_id> --step <N> --value-file <file> --db <path> [--emit text|yaml|postcard]
  graph      <workflow.yaml> [--emit text|yaml|postcard]
  diff       <run_a> <run_b> --db <path> [--emit text|yaml|postcard]
  incident   <run_id> --db <path> [--emit text|yaml|postcard]
  submit     <workflow.yaml> --input-bin <file> --db <path> --durability <mode> [--emit text|yaml|postcard]
  simulate   <workflow.yaml> [--emit text|yaml|postcard]  (static preflight — dry-run analysis, no execution)
  system status [--profile <quick|standard|full>] [--server none] [--emit text|yaml]
  agent-context [--deliver stdout|file:<absolute-path>]
  ai-context <run_id> --db <path> [--emit text|yaml|postcard]
  help
  version

architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

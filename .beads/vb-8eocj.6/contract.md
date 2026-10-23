# vb-8eocj.6 — ForEach flat-body slice (incremental)
Author: actual local5090 V222; assignee Lewis.
Status: CONTRACT ACCEPTED for the bounded implementation only; implementation/tests/CODE/TEST review remain pending.
Scope: direct declared list<any> inputs only; typed element schemas remain parent F1.

## CLOSED INPUT DOMAIN
  Accept only direct declared $input.NAME with schema {is:list, of:any, max:N}.
  N is existing schema max integer, nonnegative, checked u32 conversion.
  N=0 is empty-only. Typed element schemas (number/boolean/text/object) and
  shorthand schemas are rejected deterministically; full typed element
  admission remains parent F1. InputSlotKind::List does not retain per-element
  shape; of:any means every SlotValue is legal and item ExprTypeUnknown is
  honest. This is a deliberate temporary source coverage limit.

## PROPOSED COLD BOUND METADATA
  Immutable direct-input declaration-origin map seeded before scratch
  allocation, keyed by resolved actual input SlotIdx (bound to original
  declaration name), storing N and Any item type. No default N, no implicit
  MAX, no alias/copy/transform producer propagation; all non-direct/untracked
  references reject. Read-only input slots never overwritten. ForEachStart.limit=N,
  never at_once.

## CONCURRENCY CLOSED
  at_once omitted or 1 (serial) supported. at_once=0 or at_once>1 -> CompileError::StepFieldShape
  {step:index, field:"for_each.at_once", expected:"omitted or 1 until concurrent
  for_each lowering is implemented"}. N=0 is cardinality, not at_once=0.

## EXACT PROPOSED DIAGNOSTICS
  Wrong shape / unbounded / numeric / derived input -> StepFieldShape
  {step:index, field:"for_each.input", expected:"direct declared list<any> input
  with finite schema max"}. Unknown named input -> UnknownReferenceName.
  Typed list element schema other than any -> same StepFieldShape.
  max > u32::MAX -> InvalidInputSchema {field:"inputs.max", expected:"list maximum
  representable as u32"}. Invalid variable -> InvalidName {field:"for_each.variable",
  value:actual}. Isolated item binding collision -> DuplicateOutputName {name:variable}.
  Isolated ordinary duplicate body IDs -> DuplicateStepId {id:actual}.
  Precedence: existing schemas, body shape/layout, named resolution/domain,
  concurrency/name/collisions before node emission.

## LEXICAL NAME DECISION
  Reuse actual validate_public_name(field,value): lower ASCII <=64 + RESERVED_NAMES.
  Forbid exact singular step root if shared validator allows it. RESERVED_NAMES
  includes literal "item"; it remains invalid here. Legal example variable "entry"
  exposes $entry via actual direct resolver bare key. Do NOT globally unreserve
  "item" or claim universal $item. Require no capture/collision of item binding
  with inherited outputs/step IDs or body set output/body ID. Parent maps
  cloned/immutable; body local bindings don't escape.

## UNKNOWN TYPE DECISION
  Item is ExprTypeUnknown. Do results Unknown unless actual declared contract
  typing wired (separate future). For NEW canonical ForEach body ONLY introduce
  explicit scope-local strict compound operand policy before expression bytecode
  emission: resolve each reference with actual resolver, use evolving local type
  environment, reject compound arithmetic/boolean/comparison/helper expression
  with Unknown operand via CompileError::TypeMismatch {field:"set.value",
  expected:"known static operand type", found:"unknown"}. Opaque direct reference
  Set copies and Do inputs allowed, propagate Unknown. No fake I64/typed list
  guarantee. Known literal/outer typed scalar expressions still share existing
  type checker. Ordinary unknown name fails UnknownReferenceName first.

## LAYOUT / OWNERSHIP
  N_body >= 1. Start at S, body at S+1..S+N_body, Next at S+N_body+1, done at
  S+N_body+2 (actual next top-level). All checked usize/u16 arithmetic, no casts.
  Start.output fresh iterator; fresh item; fresh output per syntactic body target.
  Next.output item, iterator refs same private. Earlier body.next -> next body,
  last body.next -> Next, Start/Next.body -> first body; both done -> actual
  following step. Shared emit_scoped_flat_body supports Set/Do with scoped refs;
  controls/nested primitives reject unchanged. No extra result/Join claim.
  Slots allocated once/reused across iterations; lexical non-escape NOT physical
  zeroing. Real runtime body entry re-arms ready/completion/value evidence and
  refreshes item; body must actually execute again.

## FUTURE ACCEPTANCE (PENDING)
  public compile_workflow(&[u8] authored_source) -> unchanged compiled workflow
  -> shared actual runtime. Direct list of any max 3, at_once 1: len 0/1/3 exact
  iterations; max 0 empty accept / nonempty exact reject; len 4 max 3 ->
  EngineError::IterationLimitExceeded {resource:"for_each_limit"} before any body
  action. Two dependent opaque Set copies of $entry, actual registered Do using
  last copy, exact ordered SlotValue/taint inputs/results/effect observations.
  Runtime registered normal no-retry action with opaque/Any-compatible input
  contract; each iteration actually ticketed/suspends/completes, distinct per
  iteration tickets/current run identity. Do results need not be statically
  typed/cast. Mixed SlotValue Any list valid; Unknown compound arithmetic rejected
  at source; unknown names/lexical escape/collisions/overflow/concurrency negative
  precise. Input list + taint preserved; body outputs recomputed in same private
  slots. Changing N changes actual IR limit, semantic digest, whole budget
  independent A.

PARENT F1 OPEN (NOT IN THIS SLICE)
  Typed list element admission/schema artifact plumbing, of:number/boolean/text/object
  support, ordered results/Join, concurrency >1, derived lists/arbitrary nested
  controls, source action names, secrets/metadata/results/examples/deploy smoke
  rollback. Do not mark .6 complete or authorize implementation until independent
  review of exact contract.

## Delivery qualifications

Root mechanically normalized local5090 V222 output and made its requested A=0 rejection explicit. Shared default resource/boundedness validation remains mandatory; a large explicit maximum does not bypass policy. Existing capability/idempotency policy remains in force for real Do acceptance, with a bounded no-retry fixture and precise undeclared-action/capability failure before effects. This adds no retry or recovery claim.

Source-fixture migrations must retain original ownership/layout/error intent with valid bounded source, including intentional use of a legal variable such as entry. No hand-built IR replacement, manual PC resets, ignored tests, weaker assertions, or gate waivers. Implementation, behavioral tests, independent CODE review and independent TEST review are PENDING. Rust contract -> implementation -> tests -> CODE review -> TEST review; no Go-skill/full proof-planning stage.

## Independent contract disposition

Actual RTX3090 reviewed exact contract 9d8a38da79df032d72861822a83e262879072b57, SHA-256 ea4fc2a0b012cb51d7f18f3fb2853d7db85e3d9c13b81d67fb38f55dfbf4f984, via local11001/xhigh in152.387s. Verdict APPROVE_CONTRACT_ONLY with no blocking findings. The original report remains local at /home/lewis/Documents/Codex/2026-10-22/task/output/velvet-ballistics-review/foreach-contract-v222-local3090/review-report.txt; publication is not claimed. Approval does not establish implementation, native execution, proof closure, or F1 completion.

Coordinator compatibility fence added after that model request: Any-only/finite-u32 eligibility applies only to the declaration actually selected by canonical ForEach. Do not globally reject typed, unbounded, or large-max list declarations used elsewhere merely while collecting cold metadata. Retain deferred ineligibility/overflow diagnostics or inspect selected declarations only; selected-loop overflow still rejects exactly. This qualification is not attributed to the earlier model packet.

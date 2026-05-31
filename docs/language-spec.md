# velvet-ballistics Workflow Language v1

Status: draft
Canonical workflow version: `velvet-ballistics/v1`
Canonical action manifest version: `velvet/action/v1`
Suggested file extensions: `.velvet.yaml`, `.vb.yaml`, `.workflow.yaml`

`velvet-ballistics` is a strict YAML workflow language and durable execution model for high-performance workflow orchestration.

The language is built around one sentence:

```text
When this happens, take this input, run these steps, return this result.
```

YAML is the authoring source of truth. Runtime execution never interprets YAML directly; implementations compile YAML into normalized workflow IR and execute that IR against a durable event journal.

```text
YAML document
  -> restricted YAML parser
  -> workflow AST
  -> validator
  -> normalized workflow IR
  -> immutable workflow snapshot
  -> durable runtime execution
  -> event journal
  -> inspectable run state
```

## Product Naming

Product, binary, and package name:

```text
velvet-ballistics
```

Rust crate/module prefix:

```text
velvet_ballistics
```

Language version:

```text
velvet-ballistics/v1
```

CLI name:

```bash
velvet-ballistics
```

## Core Concepts

| Concept | Meaning |
| --- | --- |
| `when` | What starts the workflow. |
| `inputs` | Runtime data mapped into stable typed names. |
| `vars` | Static non-secret constants. |
| `secrets` | Required secret names, never literal secret values. |
| `steps` | Ordered actions and control primitives. |
| `result` | Final workflow output mapping. |
| `examples` | Executable fixtures for tests and dry-runs. |

Each step has an `id`. After a step succeeds, its immutable output is available as `$step_id.field`.

```yaml
version: velvet-ballistics/v1
name: hello_world

when:
  manual: {}

steps:
  - id: greeting
    save:
      text: Hello from velvet-ballistics

result:
  message: $greeting.text
```

## Design Goals

`velvet-ballistics/v1` is designed to be strict, small, typed, durable, inspectable, AI-generatable, AI-debuggable, graph-renderable, high-performance, single-binary friendly, Docker-optional, and Kubernetes-optional.

The language favors explicit bounded primitives over arbitrary code and arbitrary cycles.

## Non-Goals

v1 does not support arbitrary scripting in YAML, backward graph jumps, unbounded loops, unbounded retries, unbounded pagination, global mutable variables, hidden view-only semantics, exactly-once external side effects, or Docker/Kubernetes as language requirements.

Shell execution is allowed only as a registered action, usually:

```yaml
run: shell.run
```

Shell is not a first-class language feature.

## YAML Profile

Allowed YAML values:

```text
strings
numbers
booleans
null
lists
objects
comments
```

Rejected YAML features:

```text
duplicate keys
anchors
aliases
merge keys
custom tags
binary scalars
parser-specific YAML 1.1 booleans such as yes/no/on/off
unknown top-level fields
unknown step fields
```

The YAML parser must preserve source locations where practical: file, line, column, and YAML path. Diagnostics should include these locations.

## Value Model

Runtime values are compact, bounded values that can be projected for diagnostics:

```text
text
number
boolean
null
list handle
object handle
```

Rules:

- `number` must be finite.
- `NaN`, `Infinity`, and `-Infinity` are invalid.
- Object field names are resolved before hot runtime execution.
- Binary blobs are not inline runtime values in v1.
- Large binary/file payloads must use action-specific handles or external storage.
- Runtime output size limits apply before redaction.

## Top-Level Fields

Required top-level fields:

```yaml
version:
name:
when:
steps:
```

Optional top-level fields:

```yaml
inputs:
vars:
secrets:
result:
examples:
```

Unknown top-level fields are validation errors. `version` must be exactly `velvet-ballistics/v1`.

Valid minimal workflow:

```yaml
version: velvet-ballistics/v1
name: hello_world

when:
  manual: {}

steps:
  - id: greeting
    save:
      text: Hello

result:
  message: $greeting.text
```

Implementation coverage note: this document defines the public language shape, not a stale compiler phase subset. Current implementation coverage and gaps are governed by repo-root `velvet-ballistics-MASTER.md` plus the current coverage and mutation-plan docs.

## Names And IDs

Workflow names, step IDs, branch names, loop variables, and summary variables should match:

```text
^[a-z][a-z0-9_]{0,63}$
```

Reserved names:

```text
input
inputs
vars
secrets
steps
result
when
item
error
summary
cursor
page
event
attempt
attempts
true
false
null
run
save
choose
for_each
parallel
aggregate
gather
repeat
wait
ask
try_again
on_error
then
finish
```

All step IDs must be globally unique within one workflow, including nested scopes.

## Forbidden Legacy Aliases

v1 is strict. These legacy/internal aliases are not accepted as public YAML primitives:

| Forbidden alias | Canonical v1 primitive |
| --- | --- |
| `do` | `run` |
| `set` | `save` |
| `collect` | `gather` |
| `summarize` | `aggregate` |
| `reduce` | `aggregate` |
| `together` | `parallel` |

A future migration tool may rewrite old files into canonical v1 syntax:

```bash
velvet-ballistics migrate old.yaml --from twerk/v1 --to velvet-ballistics/v1
```

## Triggers

`when` declares exactly one trigger.

Supported v1 triggers:

```yaml
when:
  manual: {}
```

```yaml
when:
  webhook:
    path: /github
    method: POST
    unique: request.header.X-GitHub-Delivery
```

```yaml
when:
  schedule:
    cron: "*/5 * * * *"
    timezone: UTC
```

```yaml
when:
  event:
    name: customer.created
```

Trigger rules:

- Exactly one trigger is allowed.
- Unknown trigger kinds are validation errors.
- Unknown trigger fields are validation errors.
- `manual` accepts an empty object only in v1.
- Runtime trigger payloads are not directly visible inside steps.
- Trigger data must be mapped through `inputs`.
- Webhook `path` must start with `/`.
- Webhook `method` must be one of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`.
- Webhook `unique` is optional and enables dedupe-window behavior.
- Schedule cron uses five fields; seconds fields are not allowed in v1.
- Schedule `timezone` defaults to `UTC`.
- Missed schedule catch-up is disabled by default.
- Event triggers expose `event.name`, `event.id`, and `event.body` only through input mapping.

Runtime source roots by trigger:

```text
manual.user
manual.input.NAME
request.body
request.header.NAME
request.query.NAME
request.path.NAME
request.method
request.path
schedule.time
schedule.cron
schedule.timezone
event.name
event.id
event.body
```

## Inputs

Inputs map runtime trigger data into stable typed names.

Short form:

```yaml
inputs:
  email: text
  amount: number
```

Long form:

```yaml
inputs:
  email:
    from: request.body.email
    is: text

  amount:
    from: request.body.amount
    is: number
    default: 0
```

Rules:

- `inputs` must be a mapping.
- Input names must use the public name grammar.
- Each input value must be either schema shorthand or a schema mapping.
- Schema shorthand must be one of the allowed shorthand values listed below.
- Schema mappings must declare `is` and may only use supported schema fields.
- Nested object `fields` may not use `from`; only top-level input schemas may map trigger data.
- `inputs` are evaluated before the first step.
- No silent type coercion is allowed.
- String `"123"` does not satisfy `number`.
- Missing required inputs fail the run before step execution.
- Defaults must match declared type.
- Runtime source roots depend on trigger kind.
- Direct runtime references are forbidden outside `inputs.from`.

## Type System

Simple kinds:

```text
text
number
boolean
object
list
any
```

v1 does not have a separate `integer`; use `number`.

Allowed schema shorthand:

```text
text
number
boolean
object
any
list<any>
list<text>
list<number>
list<boolean>
```

Complex lists and objects use long form.

```yaml
inputs:
  customer:
    from: request.body.customer
    is: object
    fields:
      id: text
      email: text
      spend: number
      address:
        is: object
        optional: true
        nullable: true
        fields:
          city: text
          country: text
    extra: reject
```

Object rules:

- `object` without `fields` is opaque.
- `fields` is valid only when `is: object`.
- Field names inside `fields` must use the public name grammar.
- `extra: reject` rejects unknown fields.
- `extra: allow` permits unknown fields.
- Default `extra` is `reject` for declared object fields.
- `optional: true` allows a field to be absent.
- `nullable: true` allows a field to be present with value `null`.

List rules:

- `list` without `of` is invalid.
- `of` is valid only when `is: list`.
- To intentionally allow unknown element types, use `list<any>` or `of: any`.
- List order is preserved.
- Empty lists are valid unless `min` forbids them.

Supported schema fields:

```text
is
of
fields
extra
optional
nullable
default
min
max
min_length
max_length
pattern
secret
```

`default` must match the declared schema type. `default: null` is valid only when `nullable: true` or `is: any`.

`min` and `max` are valid only for `number` and `list`; list bounds must be non-negative. `min_length` and `max_length` are valid only for `text` and must be non-negative.

`optional`, `nullable`, and `secret` must be booleans.

`pattern` is allowed only if the runtime adopts a bounded RE2-style regex engine. The current compiler rejects `pattern` until bounded regex support exists.

## Vars And Secrets

`vars` are immutable non-secret constants.

```yaml
vars:
  model: fast
  region: us-east-1
  max_score: 100
```

Vars must be a mapping. Var names must use the public name grammar. Vars cannot reference runtime data, step outputs, or secrets. Vars are included in the immutable workflow snapshot.

`secrets` declare required secret bindings.

```yaml
secrets:
  github_token: GITHUB_TOKEN
  slack_webhook: SLACK_WEBHOOK
```

Secret rules:

- `secrets` must be a mapping from public secret names to environment/provider binding names.
- Secret binding names must be strings.
- Workflow files never contain literal secret values.
- Undeclared secret references are validation errors.
- Missing required secrets fail before observable use.
- Secrets are redacted from logs, traces, events, bundles, examples, debug previews, and errors.
- Any value derived from a secret becomes secret-tainted.
- Secret-tainted values are blocked from `result` by default.
- Secret-tainted values may be passed to action input fields marked `secret: true`.
- Non-secret action input fields reject secret-tainted values unless the action manifest explicitly permits taint.

## References

Allowed reference roots:

```text
$input.x
$vars.x
$secrets.x
$step_id.x
$loop_var.x
$summary.x
$error.x
$attempt.x
$attempts.x
$cursor
$page.x
$event.x
```

Examples:

```yaml
with:
  email: $input.email
  token: $secrets.github_token
  label: $classify.label
```

Rules:

- Missing paths are runtime errors unless used inside `exists(path)`.
- Future step references are validation errors.
- References to skipped step outputs fail at runtime.
- v1 validation rejects unconditional references to step outputs that may be skipped by `if`.
- Full scalar references preserve native type.
- References embedded in text become strings.
- Literal `$` is escaped as `$$`.
- References are not evaluated inside YAML keys.

## Expressions

Expressions are deterministic, bounded, side-effect-free, and statically analyzable.

Expressions are used in:

```
if
choose.if
repeat.until
wait.where
aggregate.update
save
result
on_error.save
```

Operators:

```text
== != > >= < <=
and or not
+ - * /
```

Predicate helpers:

```text
contains(value, needle)
starts_with(text, prefix)
ends_with(text, suffix)
has(object, key)
exists(path)
length(value)
empty(value)
```

Reducer helpers:

```text
append(list, value)
append_if(list, value, condition)
merge(object, object)
sum(list, field)
count(list)
unique(list)
```

Informal expression grammar:

```text
Expr        = Or
Or          = And ("or" And)*
And         = Compare ("and" Compare)*
Compare     = Add (("==" | "!=" | ">" | ">=" | "<" | "<=") Add)?
Add         = Mul (("+" | "-") Mul)*
Mul         = Unary (("*" | "/") Unary)*
Unary       = "not" Unary | "-" Unary | Primary
Primary     = Literal | Reference | FunctionCall | "(" Expr ")"
Function    = Name "(" Args? ")"
Args        = Expr ("," Expr)*
```

Evaluation rules:

- `and` and `or` short-circuit left to right.
- Missing references in non-evaluated short-circuit branches are not runtime errors.
- Static validation still verifies known reference roots and known step IDs.
- Runtime validation checks value types before operations.
- Arithmetic operands and results must be finite numbers.
- Division by zero is a runtime error.

Forbidden in expressions:

```text
JavaScript
Python
jq
network calls
random functions
user-defined functions
loops inside expressions
unbounded regex
```

## Steps

Allowed step fields:

```yaml
id:
name:
if:
run:
with:
save:
choose:
for_each:
parallel:
aggregate:
gather:
repeat:
wait:
ask:
finish:
try_again:
on_error:
then:
```

Every step must have exactly one primitive:

```
run
save
choose
for_each
parallel
aggregate
gather
repeat
wait
ask
finish
```

Every step must have exactly one primitive:

```text
run
save
choose
for_each
parallel
gather
aggregate
repeat
wait
ask
finish
```

Metadata/control fields are not primitives:

```text
id
name
if
with
try_again
on_error
then
```

Unknown step fields are validation errors.

Step execution model:

1. Evaluate `if`, if present.
2. Mark the step `skipped` when `if` is false.
3. Resolve inputs when the step runs.
4. Execute exactly one primitive.
5. Apply retry policy for runtime failures.
6. Run `on_error` only after retries are exhausted.
7. Record immutable output on success.
8. Move to `then` target if present, otherwise the next step.

## `if` And `then`

```yaml
- id: send_alert
  if: $input.priority == "urgent"
  run: slack.message.send
  with:
    text: $input.message
```

`if` is evaluated before the primitive. Skipped steps produce no output, `on_error` is not run, and control continues to the natural next step.

`then` is a forward success jump.

```yaml
- id: classify
  run: ai.classify
  with:
    text: $input.text
  then: route
```

`then` targets must be later steps in the same scope. Backward jumps and cross-scope jumps are invalid. `finish` steps cannot have `then`.

## `run`

`run` invokes a registered action.

```yaml
steps:
  - id: create_ticket
    run: ticket.create
    with:
      title: $input.title
      body: $input.body
```

Rules:

- `run` must reference a registered action.
- `with` validates against the action input schema.
- Unknown `with` fields are rejected unless the action schema allows extra fields.
- Action output must be an object.
- Output is validated before downstream exposure.
- Side effects are at-least-once.
- Retry-safe actions must declare idempotency behavior.

## `save`

`save` creates the current step output without I/O.

```yaml
steps:
  - id: title
    save:
      text: $input.body.issue.title
```

Rules:

- `save` performs no I/O.
- `save` is deterministic.
- `save` output must be an object.
- `save` cannot mutate inputs, vars, secrets, previous outputs, or runtime metadata.
- Retrying `save` is a validation warning or error.

## `choose`

`choose` selects exactly one branch.

```yaml
steps:
  - id: route
    choose:
      - if: $classify.label == "urgent"
        steps:
          - id: alert
            run: pager.alert
            with:
              message: $input.message
        result:
          kind: urgent
          id: $alert.id

      - otherwise: true
        steps:
          - id: ticket
            run: ticket.create
            with:
              body: $input.message
        result:
          kind: ticket
          id: $ticket.id
```

Rules:

- Branches evaluate top to bottom.
- First true branch wins.
- `otherwise: true` is the default branch.
- Multiple `otherwise` branches are invalid.
- No match without `otherwise` is a runtime error.
- Unselected branch outputs are inaccessible.
- `choose` output is the selected branch `result`.
- v1 requires all branch result objects to have the same top-level keys.

## `for_each`

`for_each` repeats work over a finite list.

Short action form:

```yaml
steps:
  - id: notify_all
    for_each:
      in: $input.customers
      as: customer
      at_once: 10
      per_second: 50
      run: email.send
      with:
        to: $customer.email
        subject: Hello
```

Expanded form:

```yaml
steps:
  - id: notify_all
    for_each:
      in: $input.customers
      as: customer
      at_once: 10
      steps:
        - id: send_email
          run: email.send
          with:
            to: $customer.email
            subject: Hello
      result:
        email: $customer.email
        sent_id: $send_email.id
```

Rules:

- `in` must resolve to a list.
- Empty list succeeds with `[]`.
- `as` names the scoped loop variable.
- Output order must match input order.
- Runtime may execute iterations concurrently.
- `at_once` limits in-flight iterations.
- `per_second` limits iteration starts.
- Default failure behavior is fail-fast.
- Partial iteration output is available only inside `on_error` as `$error.partial`.
- Nested step IDs are globally unique, but outputs are exported only through `for_each.result`.

## `parallel`

`parallel` runs named branches concurrently.

```yaml
steps:
  - id: enrich
    parallel:
      fail: after_all
      branches:
        profile:
          run: profile.lookup
          with:
            email: $input.email

        orders:
          run: order.list
          with:
            customer_id: $input.customer_id
```

Failure modes:

| Mode | Meaning |
| --- | --- |
| `fast` | Cancel unfinished branches after first failure and fail parent. |
| `after_all` | Let all branches finish, then fail parent if any branch failed. |
| `record` | Parent succeeds and returns success/error per branch. |

Rules:

- Branch names must be unique and ID-like.
- Branch outputs are keyed by branch name.
- Output shape is deterministic by YAML order, not completion order.
- Successful branch outputs are recorded even when parent fails.
- Parent output is unavailable if parent fails, except inside `on_error` as `$error.partial`.
- Successful side effects are not rolled back automatically.
- Compensation must be explicit.

## `gather`

`gather` handles bounded cursor pagination.

```yaml
steps:
  - id: customers
    gather:
      cursor:
        start: null
        next: $page.body.next_cursor

      page:
        run: http.get
        with:
          url: https://api.example.com/customers
          query:
            cursor: $cursor
            limit: 100

      take: $page.body.customers
      stop: $page.body.next_cursor == null

      limit:
        pages: 500
        items: 50000
        time: 5m
        wait_between: 100ms
```

Output:

```yaml
result:
  customers: $customers.items
  page_count: $customers.pages
  customer_count: $customers.count
```

Local roots inside `gather`:

```text
$cursor
$page
```

Rules:

- Unbounded pagination is forbidden.
- `gather.limit` is required.
- Cursor state is durable.
- Each page attempt is recorded.
- Items are appended to a durable scoped accumulator.
- Output appears only after successful completion.
- Partial output is available only inside `on_error` through `$error.partial`.
- Page, item, time, and wait limits are enforced by runtime.
- Step-level `try_again` retries the current page attempt, not completed pages.

## `aggregate`

`aggregate` accumulates over a finite list.

```yaml
steps:
  - id: totals
    aggregate:
      in: $customers.items
      as: customer
      start:
        count: 0
        revenue: 0
        vip_customers: []
      update:
        count: $summary.count + 1
        revenue: $summary.revenue + $customer.spend
        vip_customers: append_if(
          $summary.vip_customers,
          $customer,
          $customer.spend > 1000
        )
```

Local roots:

```
$summary
$customer
```

Rules:

- `$summary` exists only inside `aggregate`.
- `$summary` is immutable per iteration.
- Each iteration creates a new accumulator version.
- Accumulator state is durably checkpointed.
- Output appears only when `aggregate` succeeds.
- Partial accumulator is available only to `on_error` as `$error.partial`.

## `repeat`

`repeat` handles bounded polling and durable checking.

```yaml
steps:
  - id: job
    run: api.start_job
    with:
      payload: $input.payload

  - id: wait_for_job
    repeat:
      run: http.get
      with:
        url: "https://api.example.com/jobs/$job.id"

      until: $attempt.body.status == "done"

      limit:
        times: 60
        time: 10m
        wait_between: 5s
```

Local roots:

```text
$attempt
$attempts
```

Rules:

- `repeat.limit` is required.
- Each attempt is durable.
- Completed prior workflow steps are not rerun after restart.
- Output is the final successful attempt.
- Failure exposes partial attempt history in `$error.partial`.
- Step-level `try_again` retries the current repeated attempt, not the whole repeat history.

## `wait`

`wait` pauses execution durably.

```yaml
steps:
  - id: cool_down
    wait: 10m
```

```yaml
steps:
  - id: wait_until
    wait:
      until: "2026-04-28T12:00:00Z"
```

```yaml
steps:
  - id: wait_for_payment
    wait:
      for: payment.completed
      where: $event.body.customer_id == $input.customer_id
      timeout: 1h
```

Rules:

- Waits survive restart.
- Negative durations are invalid.
- Past timestamps resume immediately.
- Event waits must declare `timeout`.
- `$event` exists only in event wait `where` evaluation and wait output.
- Max wait is runtime policy.

## `ask`

`ask` creates a durable human input point.

```yaml
steps:
  - id: approval
    ask:
      question: Approve production deploy?
      choices:
        - approve
        - reject
      timeout: 24h
```

Rules:

- Prompt is durable.
- Response is untrusted input.
- Response must be validated.
- Audit trail is required.
- Self-approval is runtime policy.
- Secrets in prompts are forbidden by default.
- Timeout produces `ASK_TIMEOUT` unless `default_on_timeout` is declared.

## `finish`

`finish` terminates the run.

```yaml
steps:
  - id: done
    finish: success
```

```yaml
steps:
  - id: failed
    finish:
      status: failure
      error: deploy_failed
      message: Deploy failed
```

Allowed statuses:

```text
success
failure
cancelled
```

Rules:

- Multiple `finish` steps are allowed as alternative terminals.
- One run cannot reach two terminal steps.
- `finish` cannot have `then`.
- If `finish.status` is `success` and `finish.result` exists, that result is used.
- If `finish.status` is `success` and `finish.result` is omitted, top-level `result` is evaluated.
- Failure and cancellation do not evaluate top-level `result`.

## `try_again`

`try_again` retries the current step primitive after runtime failures.

```yaml
steps:
  - id: call_api
    run: http.post
    with:
      url: https://api.example.com/events
      body: $input.body
    try_again:
      times: 3
      when:
        - Http.Timeout
        - Http.RateLimited
      wait:
        type: exponential
        initial: 100ms
        max: 5s
        jitter: full
```

Rules:

- `times` is total attempts including the first.
- `times: 1` means no retry.
- Retry counters are durable.
- `on_error` runs only after retries are exhausted.
- Static validation errors are not retryable.
- Non-retry-safe actions require idempotency keys or runtime rejection.
- Retry policies on deterministic `save` steps are warnings or errors.
- For `gather`, retry applies to the current page attempt.
- For `repeat`, retry applies to the current repeated attempt.
- For `for_each`, retry applies to the current item execution.

Retry wait strategies:

```text
fixed
linear
exponential
```

Jitter:

```text
none
full
equal
```

## `on_error`

`on_error` handles runtime step failures after retries are exhausted.

Short form:

```yaml
on_error: failed
```

Equivalent:

```yaml
on_error:
  then: failed
```

Fallback output:

```yaml
on_error:
  save:
    found: false
    name: null
```

Terminal failure:

```yaml
on_error:
  finish:
    status: failure
    error: api_failed
```

v1 supports exactly one of:

```text
then
save
finish
```

Rules:

- Static validation errors are not catchable.
- Cancellation is not catchable by default.
- `$error` is available only inside the handler.
- Recursive error handlers are forbidden.
- Handler failure records both original and handler error.
- `on_error.save` becomes the failed step replacement output.
- After `on_error.save`, control follows the step success edge.
- `on_error.then` jumps to a forward step in the same scope.
- `on_error.finish` terminates the run.

## Top-Level `result`

`result` is the final workflow output mapping.

```yaml
result:
  label: $classify.label
  confidence: $classify.confidence
```

Rules:

- Omitted `result` defaults to `{}`.
- Missing references fail completion.
- Skipped references fail completion.
- Secret references and secret-tainted values fail by default.
- Result size is limited by runtime policy.
- Result values preserve native types.

## Examples

Examples are executable fixtures.

```yaml
examples:
  - name: bug_report
    input:
      body:
        issue:
          title: Crash on login
    expect:
      result:
        label: bug
```

Rules:

- `examples` must be a list of mappings.
- Each example must have a public `name`.
- Examples must validate against `inputs`.
- Real secrets are forbidden.
- Fake example secrets are still masked.
- Examples should be runnable through validation, compilation, and volatile execution fixtures, for example `velvet-ballistics validate flow.yaml` followed by `velvet-ballistics run flow.yaml --input-bin input.vbin --durability volatile`.

## Duration Grammar

v1 durations use one unit.

```text
<number>ms
<number>s
<number>m
<number>h
<number>d
```

Compound, negative, and fractional durations are invalid in v1. Runtime policy may impose maximums.

## State Isolation

| Scope | State rule |
| --- | --- |
| Workflow | Immutable inputs, vars, and secrets. |
| Step | Immutable output after success. |
| Choose branch | Isolated branch scope. |
| Together branch | Isolated branch scope. |
| For each item | Isolated item scope. |
| Gather | Scoped durable cursor and accumulator. |
| Summarize | Scoped durable accumulator. |
| Repeat | Scoped durable attempt history. |
| Error handler | Scoped `$error` object. |

No primitive writes to global mutable state.

## Control Flow

Rules:

- Steps run top to bottom by default.
- `then` can jump only forward to an existing step in the same scope.
- Backward jumps are forbidden.
- Arbitrary graph cycles are forbidden.
- Jumping into or out of nested scopes is invalid.
- Unreachable steps are validation errors.
- A reachable `finish` terminates the run.
- Structured repetition is provided by `for_each`, `gather`, `aggregate`, and `repeat`.

## Runtime Guarantees

The runtime must guarantee:

```text
accepted means durable
workflow snapshots are immutable
completed steps do not rerun during normal replay
step outputs are immutable
waits survive restart
asks survive restart
retries are durable
loop state is durable
pagination state is durable
parallel branch state is durable
external side effects are at-least-once
external side effects are not exactly-once
```

Accepted means: if the runtime returns a run ID, the run admission event and workflow snapshot binding are persisted.

## Run And Step States

Allowed run states:

```text
accepted
queued
running
waiting
asking
succeeded
failed
cancelled
```

Allowed step states:

```text
pending
skipped
running
waiting
asking
retrying
succeeded
failed
cancelled
```

Terminal run states are `succeeded`, `failed`, and `cancelled`. Completed steps must not rerun during normal replay.

## Error Object

Runtime errors use a stable object shape:

```json
{
  "code": "ACTION_FAILED",
  "message": "Action failed",
  "step": "classify",
  "retryable": true,
  "details": {}
}
```

Error objects must redact secrets.

## Validation Must Reject

Validation errors include:

```text
duplicate YAML keys
unknown top-level fields
unknown step fields
missing required top-level fields
invalid version
invalid workflow name
invalid step ID
duplicate step ID
multiple step primitives
missing step primitive
legacy primitive aliases
unknown references
future step references
undeclared secrets
direct runtime references outside inputs.from
invalid then targets
control-flow cycles
unreachable steps
invalid run
invalid save
invalid choose
invalid for_each
invalid parallel
invalid gather
invalid aggregate
invalid repeat
invalid wait
invalid ask
invalid finish
invalid retry policy
invalid on_error handler
secret result leaks
type mismatches
payload limit violations
unknown action names
invalid action inputs
retry-unsafe action retry
denied capabilities
```

Warnings include unused inputs, unused vars, unused secrets, unconsumed step outputs, retry policies on deterministic `save` steps, large fanout, large result payloads, shell usage, unsafe action usage, possibly skipped step output references, and missing examples.

## Validation Error Codes

Recommended validation codes:

```text
DUPLICATE_KEY
UNKNOWN_TOP_LEVEL_FIELD
UNKNOWN_STEP_FIELD
MISSING_REQUIRED_FIELD
INVALID_VERSION
INVALID_NAME
INVALID_ID
DUPLICATE_ID
MULTIPLE_STEP_PRIMITIVES
MISSING_STEP_PRIMITIVE
LEGACY_PRIMITIVE_ALIAS
UNKNOWN_REFERENCE
FUTURE_REFERENCE
SECRET_NOT_DECLARED
DIRECT_RUNTIME_REFERENCE
INVALID_THEN_TARGET
CONTROL_FLOW_CYCLE
UNREACHABLE_STEP
INVALID_RUN
INVALID_SAVE
INVALID_CHOOSE
INVALID_FOR_EACH
INVALID_TOGETHER
INVALID_GATHER
INVALID_SUMMARIZE
INVALID_REPEAT
INVALID_WAIT
INVALID_ASK
INVALID_FINISH
INVALID_RETRY
INVALID_ON_ERROR
SECRET_RESULT_LEAK
TYPE_MISMATCH
PAYLOAD_TOO_LARGE
UNKNOWN_ACTION
INVALID_ACTION_INPUT
RETRY_UNSAFE_ACTION
CAPABILITY_DENIED
```

## Runtime Error Codes

Recommended runtime codes:

```text
INPUT_MAPPING_FAILED
INPUT_TYPE_MISMATCH
SECRET_UNAVAILABLE
REFERENCE_MISSING
STEP_SKIPPED_REFERENCE
ACTION_FAILED
ACTION_TIMEOUT
ACTION_VERSION_UNAVAILABLE
RETRY_EXHAUSTED
WAIT_TIMEOUT
ASK_TIMEOUT
FOR_EACH_ITEM_FAILED
TOGETHER_BRANCH_FAILED
GATHER_LIMIT_REACHED
GATHER_PAGE_FAILED
SUMMARIZE_ITEM_FAILED
REPEAT_LIMIT_REACHED
RESULT_REFERENCE_MISSING
PAYLOAD_TOO_LARGE
QUEUE_FULL
CAPABILITY_DENIED
RUN_CANCELLED
INTERNAL_ERROR
```

## Safety Limits

Suggested defaults:

| Item | Limit |
| --- | ---: |
| YAML size | 1 MiB |
| Steps | 1000 |
| Nesting depth | 8 |
| Input after mapping | 1 MiB |
| Step output | 256 KiB |
| Result | 256 KiB |
| Parallel branches | 100 |
| Loop items | 10000 |
| Gather pages | 500 |
| Gather items | 50000 |
| Retry attempts | 10 |
| Ask timeout | 30d |
| Wait duration | 30d |

Runtime policy may be stricter.

## Action Registry Integration

Every `run` action must resolve to an action manifest.

Action manifest minimum:

```yaml
version: velvet/action/v1
name: http.get
action_version: 1.0.0
title: HTTP GET
description: Performs an HTTP GET request.

inputs:
  is: object
  fields:
    url: text
    headers:
      is: object
      optional: true
      extra: allow
  extra: reject

outputs:
  is: object
  fields:
    status: number
    body: any
    headers:
      is: object
      extra: allow
  extra: reject

side_effect: external_read

retry:
  safety: idempotent
  retryable_errors:
    - Http.Timeout
    - Http.RateLimited

timeout:
  default: 30s
  max: 2m

capabilities:
  - network.any

mock:
  mode: contract_only

ui:
  category: HTTP
  icon: globe
  color: blue
```

Action side-effect classes:

```text
pure
local_read
local_write
external_read
external_write
process
unsafe_shell
```

Retry safety:

```text
idempotent
requires_idempotency_key
not_retry_safe
unknown
```

The workflow validator must use action manifests to check unknown action names, input schemas, output reference existence, secret taint, retry safety, capability availability, and side-effect warnings.

## CLI Contract

Core commands:

```bash
velvet-ballistics validate flow.yaml
velvet-ballistics compile flow.yaml --emit ir --out flow.vbir
velvet-ballistics graph flow.yaml --emit yaml
velvet-ballistics run flow.yaml --input-bin input.vbin --durability volatile
velvet-ballistics run-compiled flow.vbir --input-bin input.vbin --durability volatile
velvet-ballistics inspect <run_id> --db <path> --emit yaml
velvet-ballistics events <run_id> --db <path> --emit yaml
velvet-ballistics trace <run_id> --db <path> --emit yaml
velvet-ballistics replay <run_id> --db <path> --emit yaml
velvet-ballistics action list --emit yaml
velvet-ballistics action inspect <action-name> --emit yaml
velvet-ballistics doctor --db <path> --emit yaml
```

Machine output rules:

- `--emit yaml` emits one structured YAML document for cold operator output.
- `--emit postcard` emits compact binary output where supported.
- No ANSI color in machine mode.
- No progress spinners in machine mode.
- No interactive prompts in machine mode.
- Errors must be structured and redacted in machine mode.
- Secrets are redacted by default.

## Event Journal

The event journal is the runtime source of truth.

Common event names:

```text
run.accepted
run.queued
run.started
run.waiting
run.asking
run.succeeded
run.failed
run.cancelled
step.pending
step.skipped
step.started
step.retrying
step.waiting
step.asking
step.succeeded
step.failed
step.cancelled
repeat.attempt.started
repeat.attempt.succeeded
parallel.branch.started
parallel.branch.succeeded
parallel.branch.failed
timer.scheduled
timer.fired
journal.commit
queue.full
```

Every event should include:

```json
{
  "event": "step.succeeded",
  "seq": 12,
  "time": "2026-04-28T12:00:00Z",
  "run_id": "run_01h",
  "workflow": "issue_triage",
  "workflow_digest": "sha256:..."
}
```

## Example: Webhook Issue Triage

```yaml
version: velvet-ballistics/v1
name: issue_triage

when:
  webhook:
    path: /github
    method: POST
    unique: request.header.X-GitHub-Delivery

inputs:
  body:
    from: request.body
    is: object
    extra: allow

  delivery_id:
    from: request.header.X-GitHub-Delivery
    is: text

secrets:
  github_token: GITHUB_TOKEN

steps:
  - id: title
    save:
      text: $input.body.issue.title

  - id: classify
    run: ai.classify
    with:
      text: $title.text
    try_again:
      times: 3
      when:
        - Ai.Timeout
        - Ai.RateLimited
      wait:
        type: exponential
        initial: 100ms
        max: 5s
        jitter: full

  - id: route
    choose:
      - if: $classify.label == "urgent"
        steps:
          - id: alert
            run: pager.alert
            with:
              message: $title.text
        result:
          kind: urgent
          id: $alert.id

      - otherwise: true
        steps:
          - id: comment
            run: github.issue.comment
            with:
              token: $secrets.github_token
              issue: $input.body.issue.number
              body: "Classified as $classify.label"
        result:
          kind: comment
          id: $comment.id

result:
  label: $classify.label
  route: $route.kind
  id: $route.id

examples:
  - name: urgent_issue
    input:
      body:
        issue:
          title: Production down
          number: 123
      delivery_id: fake-delivery-id
    expect:
      result:
        route: urgent
```

## Example: Customer Report

```yaml
version: velvet-ballistics/v1
name: customer_report

when:
  manual: {}

inputs:
  account_id: text

steps:
  - id: customers
    gather:
      cursor:
        start: null
        next: $page.body.next_cursor

      page:
        run: http.get
        with:
          url: https://api.example.com/customers
          query:
            account_id: $input.account_id
            cursor: $cursor
            limit: 100

      take: $page.body.customers
      stop: $page.body.next_cursor == null

      limit:
        pages: 500
        items: 50000
        time: 5m
        wait_between: 100ms

  - id: totals
    aggregate:
      in: $customers.items
      as: customer
      start:
        count: 0
        revenue: 0
        vip_customers: []

      update:
        count: $summary.count + 1
        revenue: $summary.revenue + $customer.spend
        vip_customers: append_if(
          $summary.vip_customers,
          $customer,
          $customer.spend > 1000
        )

result:
  customer_count: $totals.count
  revenue: $totals.revenue
  vip_customers: $totals.vip_customers
```

## High-Performance Runtime Implementation Profile

This section is not language semantics. It is the recommended implementation strategy for a performance-optimized Rust runtime.

Core architecture:

```text
YAML parser
  -> AST
  -> validator
  -> normalized IR
  -> immutable snapshot
  -> runtime scheduler
  -> Fjall-backed event journal
  -> materialized run state
```

Use compiled workflow IR, interned IDs, compact numeric step indexes, precompiled reference paths, precompiled expression bytecode, precomputed graph edges, prevalidated action contracts, append-only event records, materialized run state, bounded queues, and batched Fjall commits.

Avoid string map lookups in hot loops, runtime YAML traversal, dynamic schema interpretation during every step, allocation per expression token, revalidating static workflow structure during runs, unbounded async task spawning, and unbounded event buffers.

Current Rust crate layout:

```text
crates/
  vb_core/
  vb_yaml/
  vb_validate/
  vb_expr/
  vb_compile/
  vb_storage/
  vb_runtime/
  vb_ipc/
  vb_cli/
  workspace_tests/
```

Recommended Fjall keyspaces:

```text
workflows
workflow_names
workflow_versions
runs
run_events
step_states
step_outputs
timers
waits
asks
logs
traces
idempotency_keys
dedupe_keys
```

Runtime loops:

```text
admission loop
ready-step scheduler
timer scanner
event-wait matcher
journal committer
cold observability projection
```

On restart, the runtime must load unfinished runs, replay/materialize journal state, restore timers, restore waits, enqueue runnable steps, and avoid rerunning completed steps.

Nightly Rust is pinned for current safe/bounded Rust governance, verifier support, Miri/model-checking hooks, benchmark-only experiments, allocator benchmarking, portable SIMD experiments, and other explicitly gated experiments. PGO, `target-cpu=native`, maxperf release gates, and generated Rust performance workflows are deferred outside the current backend milestone. Nightly features must not leak into the public language.

Recommended discipline:

```text
No unsafe in core runtime.
No nightly-only public API.
No optimization that weakens determinism.
No optimization that weakens durability.
No performance shortcut that bypasses the event journal.
```

Runtime values are compact handles and finite scalars; JSON is not a runtime-core value representation.

Expressions compile once from source string to tokens, AST, typed expression tree, and bytecode or compact executable form. Runtime evaluation uses scoped value environments and precompiled path segments.

The scheduler should mostly operate on numeric indexes:

```json
{
  "index": 3,
  "id": "classify",
  "primitive": "run",
  "success_next": 4,
  "then_next": null,
  "error_handler": null,
  "condition": null
}
```

Events should be append-only and streamable. Slow observability clients must not block workflow execution.

## v1 Implementation Order

Recommended build order:

```text
1. Restricted YAML parser
2. AST model
3. Schema normalizer
4. Reference parser
5. Expression parser/evaluator
6. Validator
7. Workflow IR compiler
8. Mermaid graph output
9. Fjall event journal
10. Run admission
11. save primitive
12. run primitive with mock action registry
13. result evaluation
14. inspect/events CLI
15. try_again
16. on_error
17. choose
18. wait
19. manual and binary IPC ingress
20. replay/recovery
21. binary operator inspection
22. for_each
23. parallel
24. repeat
25. gather
26. aggregate
27. ask
28. debug bundle export
```

## v1 North Star

The first killer demo should be:

```text
Download one binary.
Run `velvet-ballistics validate workflow.yaml`.
Compile it to accepted IR.
Run it through direct API or binary IPC.
Kill the process mid-run.
Restart it.
The run resumes.
Inspect the exact event history.
Replay from a chosen point.
Export a debug bundle.
```

The language and runtime should prove this thesis:

```text
velvet-ballistics is a truly open-source, single-binary, durable workflow orchestrator
with Step Functions clarity, bounded AI-authored workflows, and inspectable event history.
```

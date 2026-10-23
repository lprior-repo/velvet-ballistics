# vb-8eocj.2.2: Source Name Tests (Local RTX3090)

These 4 test cases verify that named reference resolution errors are raised
before arithmetic type errors. Tests are written but NOT run as part of this
commit.

## Test Case 1: Forward Alias Reference (Output Kind)

```yaml
version: velvet-ballistics/v1
name: fwd_alias
when: { manual: {} }
steps:
  - id: consumer_first
    set: { output: result, value: "$future + true" }
  - id: future_id
    set: { output: future, value: 10 }
  - id: finish_step
    finish: { result: result }
```

Expect: `CompileError::UnknownReferenceName` with `kind="output"`,
`reference="$future"`, `name="future"`.
Must NOT be `CompileError::TypeMismatch`.

## Test Case 2: Forward Explicit Step Reference (Step Kind)

```yaml
version: velvet-ballistics/v1
name: fwd_step
when: { manual: {} }
steps:
  - id: consumer_first
    set: { output: result, value: "$step.future_id + true" }
  - id: future_id
    set: { output: future, value: 10 }
  - id: finish_step
    finish: { result: result }
```

Expect: `CompileError::UnknownReferenceName` with `kind="step"`,
`reference="$step.future_id"`, `name="future_id"`.
Must NOT be `CompileError::TypeMismatch`.

## Test Case 3: Self Alias Reference (Output Kind)

```yaml
version: velvet-ballistics/v1
name: selfref
when: { manual: {} }
steps:
  - id: self_step
    set: { output: self_value, value: "$self_value + true" }
  - id: finish_step
    finish: { result: self_value }
```

Expect: `CompileError::UnknownReferenceName` with `kind="output"`,
`reference="$self_value"`, `name="self_value"`.
Must NOT be `CompileError::TypeMismatch`.

## Test Case 4: Positive - Producer Before Consumer

```yaml
version: velvet-ballistics/v1
name: fwd_ok
when: { manual: {} }
steps:
  - id: producer
    set: { output: future, value: 10 }
  - id: consumer
    set: { output: result, value: "$future + 2" }
  - id: finish_step
    finish: { result: result }
```

Expect: Success. Workflow finishes with value 12.
Verifies that name resolution succeeds when the producer comes first.

# Expression Engine

Expressions are compile-time artifacts. The runtime must never interpret expression strings.

## Current Scope

There is no general expression bytecode engine yet. Current deterministic behavior is encoded directly in IR nodes:

```text
save scalar constant
copy slot
choose boolean slot
finish slot
```

`choose` requires the condition slot to contain `Bool(true)` or `Bool(false)`.

## Target Bytecode

Future expressions compile to indexed programs addressed by `ExprIdx`. Runtime evaluation uses numeric operands and checked stack operations.

Target operation groups:

```text
load slot
load constant
load accessor
comparison
boolean logic
bounded arithmetic
deterministic helpers
```

## Error Model

Expression evaluation must return typed errors for:

```text
expression index out of bounds
stack underflow
type mismatch
division by zero
non-finite number
resource budget exhaustion
```

## Forbidden

```text
runtime string eval
JavaScript
Python
jq
network calls
time/random functions
unbounded expression loops
```

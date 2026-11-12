## Domain and Purpose

Vibe is a deterministic workflow DSL for reproducible, auditable batch processing of unstructured text. It is **not** a general-purpose programming language but a structured computation model focused on data pipelines with explicit durability guarantees.

## Architecture Overview

```hemlock
Vibe source → Compiler → VBIr → Runtime
  (DSL)       (validate) (binary) (execute)
```

- **Source**: Vibe DSL with explicit, no-implied-ownership semantics
- **Compiler**: Validates workflow structure, generates VBIr
- **VBIr**: Binary intermediate representation (not text)
- **Runtime**: Executes VBIr with deterministic replay capability

## Key Design Decisions

### 1. No Type Inference

```hemlock
// Bad: requires type inference
let x = 42;

// Good: explicit typing
let x: i64 = 42;
```

Rationale: Eliminates inference engine complexity; enables precise, documented type errors.

### 2. No Closures or Lexical Scoping

```hemlock
// Bad: closure captures environment
let f = fn() { return x; };

// Good: plain functions with declared parameters
fn compute(x: i64): i64 {
    return x * 2;
}
```

Rationale: Simplifies typechecking, prevents free variable analysis, enables straightforward stack allocation.

### 3. Deterministic Execution and Replay

```hemlock
// Good: pure function for replay
fn process(data: string): string {
    return data.to_upper();
}

// Bad: side effects not captured in function signature
fn process(data: string): string {
    let now = time();  // Not captured in function signature
    return data + " at " + now;
}
```

Rationale: Enables replay for debugging, auditing, and verification. All external calls must be explicit.

### 4. No Implied Ownership or Automatic Cleanup

```hemlock
// Good: explicit cleanup
let handle = open_file("data.txt");
let content = handle.read();
handle.close();

// Bad: relies on garbage collection or RAII
let handle = open_file("data.txt");
let content = handle.read();
// Implicit cleanup when handle goes out of scope
```

Rationale: Makes memory management explicit, prevents surprises, enables efficient stack allocation.

### 5. Explicit Concurrency Primitives

```hemlock
// Good: explicit task spawning
let task = spawn(fn() {
    return compute(42);
});
let result = join(task);

// Bad: implicit parallelism via runtime scheduler
let results = [1, 2, 3].map(compute);
```

Rationale: Prevents non-determinism from runtime scheduling, makes parallelism explicit in source.

### 6. No Exception Propagation Across Task Boundaries

```hemlock
// Good: explicit error handling in join
let task = spawn(fn() {
    throw "error";
});
try {
    let result = join(task);
} catch (e) {
    // Handle error explicitly
}

// Bad: exception propagates through join automatically
let result = join(task);  // Uncaught exception crashes program
```

Rationale: Simplifies error handling model, prevents unexpected crashes.

### 7. Limited Standard Library

```hemlock
// Good: use only stdlib functions
let result = stdlib::sha256(data);

// Bad: use external libraries or FFI
let result = c_lib::sha256(data);
```

Rationale: Ensures portability, simplifies the runtime environment.

### 8. No Closures

```hemlock
// Bad: requires environment capture
let make_adder = fn(x) {
    return fn(y) {
        return x + y;
    };
};

// Good: plain function with parameters
fn add(x: i64, y: i64): i64 {
    return x + y;
}
```

Rationale: Avoids environment capture complexity, simplifies type system.

### 9. No Object Literals

```hemlock
// Bad: object literal with inferred fields
let person = { name: "Alice", age: 30 };

// Good: define struct, then instantiate
define Person {
    name: string,
    age: i64
}
let person = Person(name: "Alice", age: 30);
```

Rationale: Enables structural typing and explicit field validation.

### 10. No Implicit Type Coercions

```hemlock
// Bad: implicit conversion
let x: i64 = 3.14;

// Good: explicit conversion
let x: i64 = i64(3.14);
```

Rationale: Prevents surprising precision loss or type changes.

## Error Handling

```hemlock
try {
    let result = compute();
} catch (e) {
    // Handle error explicitly
}
```

- No try/finally or try/catch/finally
- All exception handling must be explicit
- No automatic resource cleanup on exception

## Type System

```hemlock
// Nominal typing with structural subtyping
define Animal {
    name: string,
    legs: i64
}

define Dog extends Animal {
    breed: string
}

let animal: Animal = Dog(name: "Fido", legs: 4, breed: "Labrador");
```

- Nominal types with structural subtyping for compatibility
- No union types or tagged unions
- No type annotations or inference

## Concurrency Model

```hemlock
// Task-based parallelism with channels
let channel = channel(10);

let producer = spawn(fn() {
    channel.send(42);
});

let consumer = spawn(fn() {
    let value = channel.recv();
    return value;
});

join(producer);
let result = join(consumer);
```

- Task-based parallelism with explicit task spawning and joining
- Channels for communication
- No automatic parallelism or work-stealing

## Standard Library Philosophy

```hemlock
// Only what's needed for reproducible workflows
let hash = stdlib::sha256(data);
let now = stdlib::time();
let random = stdlib::random_bytes(32);
```

- Focus on deterministic operations
- Explicit external calls
- No FFI or dynamic linking

## What Vibe Is Not

- **Not a general-purpose language**: No GUI, no networking, no system calls
- **Not a scripting language**: No dynamic typing, no REPL
- **Not a systems language**: No pointers, no manual memory management
- **Not a concurrent language by default**: Concurrency is explicit, not implicit
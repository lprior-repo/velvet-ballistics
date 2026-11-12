---
section: 22
title: "Numeric IR Contract"
parent: velvet-ballistics-MASTER.md
---

## 22. Numeric IR Contract

The compiler lowers SDK DSL into numeric IR. Runtime executes only IR.

Preferred v2 bytecode core:

```rust
#[repr(u8)]
pub enum Op {
    Nop,
    Const,
    Copy,
    Eval,
    Build,
    Branch,
    Jump,
    Action,
    Wait,
    Ask,
    Finish,
}

#[repr(C)]
pub struct Instr {
    pub op: Op,
    pub flags: u8,
    pub out: u16,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub next: u16,
}
```

Rich workflow constructs lower into a small instruction set plus side tables:

```text
expr_table
build_table
branch_table
action_table
wait_table
ask_table
loop_table
const_table
accessor_table
source_map_table
```

The runtime does not know SDK syntax. It executes verified numeric instructions until blocked, finished, failed, cancelled, or budget-exhausted.

---


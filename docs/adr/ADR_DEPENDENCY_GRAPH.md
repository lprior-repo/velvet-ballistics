# ADR Dependency Graph

This graph maps the current ADR dependency structure.

```mermaid
flowchart TD
  A001["001 Backend IR North Star"]
  A002["002 Naming Workspace"]
  A003["003 Rust Governance"]
  A004["004 YAML Boundary"]
  A005["005 Errors Diagnostics"]
  A006["006 Accepted Artifact IR"]
  A007["007 Slot Values Arena"]
  A008["008 Expression Engine"]
  A009["009 Taint Secrets"]
  A010["010 Whole Workflow Bounds"]
  A011["011 Node Semantics Runtime"]
  A012["012 Actions Idempotency"]
  A013["013 Fjall Journal"]
  A014["014 Recovery"]
  A015["015 Binary IPC"]
  A016["016 Runtime Admission"]
  A017["017 CLI Control Plane"]
  A018["018 Evidence Gates"]
  A019["019 Performance Evidence"]
  A020["020 Drift Register"]
  A021["021 Deferred Scope"]
  A022["022 ADR Governance"]
  A023["023 Single Server Ownership"]
  A024["024 Hot Cold Boundaries"]

  A001 --> A002
  A001 --> A003
  A001 --> A004
  A001 --> A006
  A001 --> A011
  A001 --> A013
  A001 --> A015
  A001 --> A021
  A001 --> A023

  A002 --> A022
  A003 --> A010
  A003 --> A011
  A003 --> A018
  A003 --> A019
  A003 --> A024

  A004 --> A005
  A004 --> A006
  A004 --> A008
  A004 --> A010

  A005 --> A017
  A005 --> A018
  A006 --> A007
  A006 --> A008
  A006 --> A010
  A006 --> A011
  A006 --> A013
  A006 --> A016

  A007 --> A008
  A007 --> A009
  A007 --> A011
  A008 --> A010
  A008 --> A011
  A009 --> A012
  A009 --> A016

  A010 --> A011
  A010 --> A016
  A011 --> A012
  A011 --> A013
  A011 --> A014
  A011 --> A024

  A012 --> A016
  A013 --> A014
  A013 --> A023
  A014 --> A018
  A015 --> A016
  A015 --> A017
  A016 --> A018
  A017 --> A018
  A018 --> A019
  A020 --> A018
  A021 --> A018
  A022 --> A020
  A023 --> A014
  A024 --> A011
```

## Critical Paths

| Path | Dependency chain |
|------|------------------|
| Accepted artifact path | `001 -> 004 -> 006 -> 010 -> 016 -> 018` |
| Durable recovery path | `001 -> 006 -> 011 -> 013 -> 014 -> 018` |
| Side-effect safety path | `001 -> 009 -> 012 -> 016 -> 013 -> 014` |
| IPC admission path | `001 -> 015 -> 016 -> 013 -> 005` |
| Performance truth path | `001 -> 003 -> 011 -> 024 -> 019 -> 018` |
| Scope control path | `001 -> 021 -> 022 -> 020` |

STATUS: APPROVED

No domain refactor required. Existing typed IDs (`ActionId`, `StepIdx`) and typed recovery errors make illegal proof outcomes representable as explicit `kani::assert(false, ...)` branches.

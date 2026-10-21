# Railway errors

Preserve existing typed categories where applicable: DuplicateInputName, InvalidInputSchema, UnknownOutputName/UnknownReferenceName, UnsupportedAccessorReference, SlotIndexOutOfRange and StepFieldShape. Diagnostics should identify declaration/consumer location, not mask unknown input names as integer-string parse failures.

Proposed semantic categories requiring API coordination: UnsupportedInputSchemaKey carrying declaration/key/source location (including default), UndeclaredInputSlot, DuplicateInputSlot, InputBackingMissing, InputBackingCorrupt, InputOwnershipMismatch, InvalidInputNodeReference and InputBudgetExceeded. Exact enum names are proposals, not claims that these variants exist. Do not expose submitted payload content in diagnostics. Missing-required/default-value errors are outside the active language scope.

Codec version mismatch, workflow/artifact/input digest mismatch, arena exhaustion and malformed references are separate failure rails. No error path may substitute Null, slot0, an empty list or a weaker acceptance policy.

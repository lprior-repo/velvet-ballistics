# Railway errors

Preserve existing typed categories where applicable: DuplicateInputName, InvalidInputSchema, UnknownOutputName/UnknownReferenceName, UnsupportedAccessorReference, SlotIndexOutOfRange and StepFieldShape. Diagnostics should identify declaration/consumer location, not mask unknown input names as integer-string parse failures.

Proposed semantic categories requiring API coordination: MissingRequiredInput (only after D1), InvalidInputDefault, InputDefaultKindMismatch, UndeclaredInputSlot, DuplicateInputSlot, InputBackingMissing, InputBackingCorrupt, InputOwnershipMismatch and InputBudgetExceeded. Exact enum names are proposals, not claims that these variants exist.

Codec version mismatch, workflow/artifact/input digest mismatch, arena exhaustion and malformed references are separate failure rails. No error path may substitute Null, slot0, an empty list or a weaker acceptance policy.

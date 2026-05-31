//! Validation error formatting (part 2).
        }
        ValidationError::InvalidChoose => {
            outln!("Invalid Choose");
            outln!("  The 'choose' (conditional) construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'choose' construct structure",
                    "Choose requires 'when' conditions and 'then' branches",
                ],
            );
        }
        ValidationError::InvalidForEach => {
            outln!("Invalid ForEach");
            outln!("  The 'for_each' loop construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'for_each' construct structure",
                    "ForEach requires an 'over' iterable and a 'do' body",
                ],
            );
        }
        ValidationError::InvalidTogether => {
            outln!("Invalid Together");
            outln!("  The 'together' (parallel) construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'together' construct structure",
                    "Together requires a 'do' block with parallel steps",
                ],
            );
        }
        ValidationError::InvalidCollect => {
            outln!("Invalid Collect");
            outln!("  The 'collect' pagination construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'collect' construct structure",
                    "Collect requires an 'over' iterable and pagination settings",
                ],
            );
        }
        ValidationError::InvalidReduce => {
            outln!("Invalid Reduce");
            outln!("  The 'reduce' fold construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'reduce' construct structure",
                    "Reduce requires 'over' iterable, 'initial', and 'do' body",
                ],
            );
        }
        ValidationError::InvalidRepeat => {
            outln!("Invalid Repeat");
            outln!("  The 'repeat' loop construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'repeat' construct structure",
                    "Repeat requires 'times' or 'until'/'while' conditions",
                ],
            );
        }
        ValidationError::InvalidWait => {
            outln!("Invalid Wait");
            outln!("  The 'wait' step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'wait' step structure",
                    "Wait may require a 'for' duration or 'until' condition",
                ],
            );
        }
        ValidationError::InvalidAsk => {
            outln!("Invalid Ask");
            outln!("  The 'ask' (interaction) step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'ask' step structure",
                    "Ask requires a 'prompt' and may have 'choices'",
                ],
            );
        }
        ValidationError::InvalidFinish => {
            outln!("Invalid Finish");
            outln!("  The 'finish' step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'finish' step structure",
                    "Finish may require 'result' or 'error' fields",
                ],
            );
        }
        ValidationError::InvalidRetry => {
            outln!("Invalid Retry");
            outln!("  The 'retry' construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'retry' construct structure",
                    "Retry requires 'do' body and may have 'times' or 'until'",
                ],
            );
        }
        ValidationError::InvalidOnError => {
            outln!("Invalid OnError");
            outln!("  The 'on_error' error handler is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'on_error' handler structure",
                    "OnError requires 'do' body and may have 'max_attempts'",
                ],
            );
        }
        ValidationError::SecretResultLeak => {
            outln!("Secret Result Leak");
            outln!("  A secret value may be exposed in the workflow result.");
            explain_repair_hint(
                "validation",
                &[
                    "Exclude secret values from the workflow result",
                    "Use slot references that don't expose secret data",
                ],
            );
        }
        ValidationError::TypeMismatch { expected, found } => {
            outln!("Type Mismatch");
            outln!("  Expected type: {expected}");
            outln!("  Found type: {found}");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the value type to match the expected type",
                    "Check the Velvet v1 schema for type requirements",
                ],
            );
        }
        ValidationError::PayloadTooLarge => {
            outln!("Payload Too Large");
            outln!("  The workflow payload exceeds size limits.");
            explain_repair_hint(
                "validation",
                &[
                    "Reduce the workflow size by removing unnecessary content",
                    "Split the workflow into smaller sub-workflows",
                ],
            );
        }
        ValidationError::LimitRequired { resource } => {
            outln!("Limit Required");
            outln!("  Resource '{resource}' requires an explicit limit.");
            explain_repair_hint(
                "validation",
                &[
                    "Add an explicit limit for the resource",
                    "Check the Velvet v1 schema for limit requirements",
                ],
            );
        }
        ValidationError::LimitExceeded { resource } => {
            outln!("Limit Exceeded");
            outln!("  Resource '{resource}' has exceeded its configured limit.");
            explain_repair_hint(
                "validation",
                &[
                    "Increase the resource limit or reduce consumption",
                    "Check the Velvet v1 schema for limit values",
                ],
            );
        }
        ValidationError::UnsupportedTrigger { trigger } => {
            outln!("Unsupported Trigger");
            outln!("  Trigger type '{trigger}' is not supported.");
            explain_repair_hint(
                "validation",
                &[
                    "Use a supported trigger type: manual, schedule, webhook",
                    "Check the Velvet v1 schema for supported triggers",
                ],
            );
        }
        ValidationError::HttpTriggerOutOfCore => {
            outln!("HTTP Trigger Out of Core");
            outln!("  HTTP triggers are not available in the core runtime.");
            explain_repair_hint(
                "validation",
                &[
                    "Use a different trigger type for core runtime",
                    "HTTP triggers require the extended runtime",
                ],
            );
        }

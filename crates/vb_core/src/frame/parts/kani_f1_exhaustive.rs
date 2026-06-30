    /// K-F1: All 64 (8×8) state-transition pairs validated correctly.
    #[kani::proof]
    fn validate_transition_exhaustive_64() {
        let mut errors = 0usize;
        let mut total = 0usize;

        {
            let c = StepState::Pending;
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->S");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->F");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->K");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->C");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "P->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "P->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->P");
            }
        }
        {
            let c = StepState::Running;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "R->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->S");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->F");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->K");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->W");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->A");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->C");
            }
        }
        {
            let c = StepState::Succeeded;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Failed;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Skipped;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Waiting;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "W->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "W->W");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->C!");
            }
        }
        {
            let c = StepState::Asking;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "A->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "A->A");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->C!");
            }
        }
        {
            let c = StepState::Cancelled;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "!-->!");
            }
        }

        kani::assert(total == 64, "exhaustive 64 pairs covered");
        kani::assert(errors == 0, "all 64 pairs validated correctly");
    }


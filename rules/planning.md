# Planning before implementation

Read this when the task is to produce a plan or spec rather than the change itself — a design handed to someone else, an approach agreed before editing, or an open-ended request that has to be pinned down first. Also when writing or updating an architecture decision record.

- The plan is finished when it is decision complete: whoever implements it — another person, another agent, you tomorrow — makes no further decisions. Every open choice left in the plan gets made twice, differently.
- Planning is read-only. Searching, reading, static inspection, dry runs, and builds or tests that touch only caches and generated output refine a plan; editing files, running a formatter or codegen that rewrites them, or applying a migration is executing it. Asked mid-planning for something that would execute the plan, plan that work instead — unless the user is switching the task to implementation, which their next instruction, not your reading of it, decides.
- Explore before asking, always in that order: run at least one targeted pass over the repository — entry points, configuration, schemas, call sites — before the first question. A question the repository answers spends the user's attention on something you could have read.
- Two kinds of unknowns get handled differently: a discoverable fact (where a symbol lives, which version is pinned, what the current behavior is) is researched, never asked; ask only when several candidates survive the search, and then present them with a recommendation.
- A preference or tradeoff (scope, priority, which of two acceptable designs) cannot be derived from any file — ask it early, before the plan sets around a guess.
- Every question earns its place by changing the plan, locking an assumption, or choosing between real tradeoffs. Offer 2–4 mutually exclusive options plus a recommended default, and no filler option nobody would pick.
- An unanswered optional question resolves to your recommended default, recorded as an explicit assumption in the plan. Do not stall on it, and do not silently drop the choice.
- Plan shape: a title, then three to five short sections — summary, the changes grouped by subsystem or behavior, the test plan, the assumptions. Add a scope section only where a boundary is genuinely easy to cross by mistake.
- Group changes by behavior, not by file. A file-by-file inventory reads as thorough and hides whether the behavior is covered; name a path only to disambiguate a non-obvious change, and keep it to a handful.
- Plan the capability the request asked for. Inventing schema detail, validation policy, precedence rules, or wire formats the request never raised commits the implementer to decisions nobody made — leave them out unless their absence would cause a concrete mistake.
- In a spec, each line is marked as a requirement, a constraint or a preference, and each requirement carries an acceptance check that could fail (given/when/then, or a command and its expected result). An implementer treats an unmarked preference as a requirement, and a requirement without a check is met by whatever got built.
- Trim what the implementer already knows: repeated repository facts, invariants restated per bullet, branch-by-branch logic, lists of behavior that does not change. For a routine refactor the whole plan is a summary, key edits, tests, and assumptions.
- The plan ends with the plan. No "shall I proceed?" — the user's next message is the answer, and asking makes an agreed plan look unfinished.
- A revised plan replaces the previous one in full. Publishing a delta against a plan the user has already stopped reading is how two incompatible versions end up in the same thread.
- A plan as a spec and a plan as a progress checklist are different artifacts: the checklist tracks steps you are executing, the spec is the thing handed over. Do not let the tool that renders one stand in for the other.
- No plan for work that is one obvious step, and no single-step plan: if the plan has one step, the task did not need planning.
- An accepted decision record is never edited to reflect a new decision: write a new record and mark the old one superseded by it. The chain of records is the history of why; rewriting the old one erases what was believed when the choice was made.

Not a violation — leave these alone:

- A one-line answer to a question the user asked directly. The planning discipline applies when a plan is the deliverable, not to every reply.
- An assumption stated in the plan instead of asked about, when the question was optional and the default is named. That is the rule working, not a shortcut.

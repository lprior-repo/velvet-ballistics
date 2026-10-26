# Design patterns and architecture choice

Read this when structuring a new module or service, adding state that other code reads, wiring dependencies between modules or between runtime environments, choosing or reviewing a design pattern, or naming an app architecture (MVC/MVP, layered, hexagonal).

- A pattern needs a named recurring problem. Apply one only after the simplest working code has demonstrably recurred as a problem (core coding ladder); a pattern justified by "best practice" instead of a problem is over-engineering.
- Reuse the vocabulary the codebase already speaks: identify the incumbent patterns (Repository, Active Record vs Data Mapper, Unit of Work, MVC/MVP, event bus) and stay consistent with them. A second competing pattern for the same concern is a defect even when it is "better".
- Composition over inheritance. Inheritance only for genuine is-a substitution a caller relies on; a hierarchy deeper than two levels, or a base class that exists only to share code, gets refactored to composition.
- Never implement Singleton: use a module-level instance, DI, or the platform's single-instance mechanism. The real smell is global mutable state — naming it "Singleton" doesn't fix it.
- Replace GoF machinery with the language feature that made it obsolete: first-class functions over Strategy/Command, generators over Iterator, language decorators over Decorator classes, channels/events/signals over hand-rolled Observer plumbing. A pattern class where a closure suffices is noise.
- Present a pattern decision as use-when / avoid-when with the 2–3 trade-offs that matter, and name the trade-off accepted when applying one — never justify by implementation mechanics.
- Name the code smell first, then pick the refactoring or pattern it points to — smell → refactoring → pattern, not pattern-first (`rules/refactoring.md`).
- Architecture weight matches project size, and testability decides layers: a layer earns its place by making logic reachable by a test (humble views — no domain logic in UI components). VIPER/hexagonal ceremony on a small app is a bug, not rigor.
- One interface, one implementation, no concrete second consumer on the roadmap = speculative abstraction (core YAGNI); delete it, or mark it as a deliberate, documented extension point.
- A god object is measured in fused concerns, not lines: a long file cohesive around one hard problem is fine; a short file doing routing + persistence + rendering is not.
- Pattern advice is version-bound: check the installed major before applying framework patterns from articles or training data — e.g. pre-hooks React patterns and Vue 2 patterns are anti-patterns in the current majors (core "code against the installed version").
- Derive, never mirror: a value computable from existing state, or owned by another component, is computed or re-read where it is used. Copying it into a second local value kept in sync by an observer or an effect makes two sources of truth that disagree the moment the source changes.
- Name the coupling before accepting it: a shared data shape is cheap; temporal coupling (B works only after A ran, and nothing in the types says so) is fixed by passing A's result to B; control coupling (a flag argument choosing the behavior) means the function is two functions, so split it.
- Never introduce a dependency cycle between modules: two modules that import each other are one module in two files. Break it by extracting the shared piece or inverting one edge.
- A cycle that was already there is reported as a finding, not fixed as a side effect of the task (core Coding rule).
- Code that diverges by platform, environment or runtime diverges at the leaves: one adapter per environment, selected at the boundary, behind an interface the shared code depends on. An environment conditional inside shared logic stops that logic from being shared.
- The request shape, the domain type and the storage record become separate types once their shapes diverge: a storage entity returned straight through the API turns a schema change into an API change, and a request object passed into domain logic turns an API field into a de facto column. While all three are identical, one type is enough (core YAGNI).

Not a violation — leave these alone:

- A cache, a memoized result, a read model or a denormalized column with a named invalidation path. The derive rule targets a copy nobody owns; these have an owner (`rules/caching.md`).
- An offline copy or an unsaved draft kept on purpose, which syncs back through one documented path.
- A boolean argument that selects data rather than behavior (`includeArchived`): it is a parameter, not control coupling.

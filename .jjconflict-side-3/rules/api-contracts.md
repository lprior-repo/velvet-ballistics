# API contracts and generated clients

Read this when the repo has a machine-readable API schema (OpenAPI or similar), a contract package, or generated API clients — or before hand-writing types for API data.

- The schema is the source of truth: change the spec, re-run codegen. Never hand-edit generated files — they are build output and the next run overwrites them.
- Before hand-writing a DTO or request/response type, look for an existing contract package or your ecosystem's schema-to-client generator; duplicating shapes by hand is how client and server drift.
- Schema change → regenerate in the same change and fix all consumers; a stale generated client compiles but lies.
- Generated output is deterministic: sorted keys and ids, no timestamps, absolute paths or machine names, so regenerating unchanged input is a byte-identical no-op. CI regenerates and fails on `git diff --exit-code`; a diff there is a generator defect or a stale commit, never churn to commit.
- The error envelope is part of the contract: clients parse the documented shape (`code`, `message`, `request_id`) in one shared helper, not per endpoint — parsing it in several places is how the shape drifts.
- Generate the schema from the route/handler definitions and treat it as the single source of truth; export it in CI (a headless script that boots the app and writes the spec) to feed downstream SDK/client generation.
- The response shape is part of the contract: define and serialize responses through the schema so undocumented internal fields never ship (`rules/backend-security.md`).
- gRPC/protobuf and GraphQL SDL are contracts under the same rules: the schema file is the source of truth, clients are generated, and a field removal or type change is a public API change.
- Gate breaking changes mechanically: run a schema diff in CI against the released spec (`buf breaking` for protobuf, oasdiff for OpenAPI) and fail the build on a breaking diff unless the version bumps (`rules/public-api-design.md`).
- With two or more independent consumers, add consumer-driven contract tests (Pact or a schema-based equivalent): provider CI verifies recorded consumer expectations, catching removals consumers actually rely on before deploy orders collide.
- Contract changes are public API changes — the core "Ask first" rule applies. Evolving a public HTTP surface (versioning, pagination, deprecation): `rules/public-api-design.md`.

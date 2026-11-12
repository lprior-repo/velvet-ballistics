# Deployment and release

Read this when shipping a change to a running environment or to a release or support branch: rollout strategy, rollbacks, feature flags, migration ordering, build artifacts, runtime hardening, and infrastructure definitions with their state and plans.

- Deploy and release are separate events: deploying puts code in production dark; releasing turns it on (flag, router weight). Coupling them makes every rollback a redeploy under incident pressure.
- Every deploy has a tested rollback path before it starts: the previous artifact still deployable, migrations compatible one version back. A change that can't roll back (data rewrite, destructive migration) is declared as such and ships alone, never bundled.
- Expand/contract ordering is a deploy contract, not just a schema pattern: ship code tolerating both shapes → expand/migrate → switch reads and writes → contract only after every consumer is off the old shape. It applies to API fields, event schemas, and config keys the same as columns (`rules/database.md`, `rules/messaging.md`).
- A rolling deploy runs old and new code side by side against the same database and queues — every change must be compatible with its immediate predecessor, or the deploy needs a declared maintenance window.
- Progressive rollout for risky changes: a canary or percentage stage with promote/abort metrics (error rate, latency vs baseline) watched for a stated bake time — an unwatched canary is just a slow full rollout.
- Feature flags are code with a lifecycle: each flag records an owner, a default (off for releases), and a removal condition at creation. A flag that outlives its rollout is a permanent untested branch — two code paths, one of them dark.
- A kill switch is not a release flag: it is permanent, exercised regularly, and fails safe (flag service down → a per-flag decision of last-known or safe default); a release flag is temporary by definition.
- Flag evaluation is consistent within one request/session — a mid-request flip splits a user across both code paths.
- Migrations deploy separately from, and before, the code that needs them — a step that both migrates and serves couples schema state to instance count mid-rollout (`rules/database.md`).
- Releases are tagged and their changelog and artifacts produced by CI, so every release rebuilds from its tag and no hand step is skipped or done differently (`rules/ci-cd-security.md`).
- One artifact is built once and promoted through every environment, with configuration injected at deploy. It carries only the built output and its runtime dependencies: no source, dev dependencies, build toolchain, env files or secrets. A build step that needs a secret gets a throwaway placeholder and the real value is injected at runtime, because anything baked into an artifact is recoverable from it.
- Workloads run least-privileged: an unprivileged user, a read-only filesystem with explicit writable mounts, only the OS capabilities the process needs, no privilege escalation. An application never gets the host's container-runtime or orchestrator control socket, which is root on the host.
- A declarative definition owns every workload and handles restart, rollout and placement; changes go through the committed definition, never a live edit on the running system, which drifts it from what's declared.
- Every workload declares its CPU and memory requests and limits, and a rollout or a node drain keeps enough replicas serving: tune the maximum unavailable to real capacity, since the defaults can brown out a small deployment.
- Every distributed client the backend serves (mobile, desktop, CLI, extension) is checked against a server-side minimum version, so a known-vulnerable build can be cut off.
- A fix needed on a release or support branch also lands on the mainline in the same change set, mainline first by default; an incident hotfix made on the release branch is forward-ported before the incident closes. A fix that exists only on a release branch disappears from every release cut after it.

## Infrastructure definitions

- Infrastructure state is remote with locking from the first commit. It holds secrets in plaintext, so treat it as a credential: an encrypted, access-controlled backend, never committed.
- The plan is the review artifact: apply only a plan someone read. A replacement of anything stateful (a database, a volume) is a destructive operation under the core Boundaries rule and needs the same explicit confirmation and backup as a DROP; destroy, forced replacement and state surgery are never run unprompted, and always after a state backup.
- Pin provider and module versions exactly; an unpinned module source runs with apply-time rights (`rules/dependencies.md`).
- Drift is a first-class state: detect it with a scheduled plan, then decide import or revert explicitly instead of letting the next apply decide.
- One state per environment; a shared state turns a staging experiment into a production incident.
- Secrets are referenced from the secret manager, never written as literals, since a literal lands in state and plan output.
- Import an existing resource before declaring it: a new definition colliding with a live name either fails or replaces the running resource.

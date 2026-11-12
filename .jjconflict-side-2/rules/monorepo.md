# Monorepos

Read this when working in a repository that holds several packages, modules or services built together, in any ecosystem (JS/TS workspaces, Cargo workspaces, Go workspaces, Gradle multi-project, uv workspaces, or equivalent).

- Declare a dependency in the package that uses it, never at the root; the root holds repository-level tooling only.
- Import another package through its public entry point, never by a deep path into its internals, which can change without notice.
- No new `shared`/`common`/`utils` dumping-ground packages: extend the package that owns the domain, or propose a named, scoped package.
- Run tasks through the repository's orchestrator or workspace commands so caching and dependency order apply; don't hand-run each package.
- A change to a published package carries the repository's release-note record (a changeset file, a changelog entry) in the same commit; the release tooling skips a change that has none.
- Side-effecting tasks (migrations, seeds, deploys) are never cached, and environment files are part of the task cache key, so an environment change rebuilds instead of serving a stale result.
- Expose operations as named workspace tasks (`db:migrate`, `infra:deploy`), one discoverable entry point instead of raw tool invocations contributors must memorize.
- One lockfile at the repository root covering the whole workspace, never one per package: a per-package lockfile resolves that package's tree in isolation and silently diverges from what the root install produces (`rules/dependencies.md`).
- An import that resolves is not proof the package declares it: a package manager that hoists transitive dependencies into a shared tree makes an undeclared import work until an unrelated version bump removes the hoisted copy. Check the importing package's own manifest.
- Dependencies point one way: applications depend on packages, never the reverse, or the package can no longer be reused outside this one app.
- A package meant for more than one environment (browser, server, mobile, CLI) takes an environment-specific capability through an interface the consuming application implements, instead of importing it (`rules/design-patterns.md`).

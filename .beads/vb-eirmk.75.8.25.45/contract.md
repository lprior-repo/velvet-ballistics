# Public node routing bounds

The existing public validate_node_bounds helper checks node IDs, ordinary next targets and error-handler targets against the node array length, in that order for each node. Add the missing error-handler check with the existing StepOutOfBounds payload. Preserve all current APIs, resource checks, error-slot handling and node-kind transition validation. Current checked constructors already reject bad handlers, so this is a standalone helper contract repair, not a runtime admission bypass.

Use a genuine compiler-derived two-node workflow with valid handler1/error_slot0 for both constructor controls. Test absent handler, last valid handler, exact-length and MAX bad handlers, and unchanged ID/next error precedence. No dependency, format, schema or gate changes. Independent CODE/TEST review and scoped execution required; CPU/cloud only, no benchmarks.

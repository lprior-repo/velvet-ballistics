# Contract: Honor existing MASTER advisory reporting policy without losing supply-chain failures

## Scope
Only `.moon/tasks/all.yml` `supply-chain` task. No source or proof changes. No new tooling.

## Problem
Current `supply-chain` task wraps all tools in `run_logged()` which fails the task on any nonzero exit from deny/vet/geiger/machete. This contradicts the 2026-05-23 owner waiver (MASTER lines 114, 1683, 1721, 4786) that makes these reports advisory. Actual `cargo audit` vulnerabilities were fixed earlier and must remain fail-closed.

## Acceptance Criteria

### Preserve audit fail-closed
- `cargo audit` finding actual vulnerabilities still fails the task (nonzero exit)
- No advisory allowlists added; findings not suppressed

### Advisory reports capture and continue
- `cargo deny check`, `cargo vet`, `cargo geiger`, `cargo machete` run and capture output
- Each advisory tool's raw stdout and stderr captured in named files
- Each advisory tool's exact exit status recorded in a status file
- Advisory tool failure does NOT fail the overall task
- Nonzero advisory report status remains nonzero in metadata (never relabeled PASS)

### Conspicuous statuses for timeouts and missing tools
- Missing tools exit 127, captured and recorded
- Timeouts exit 124, captured and recorded
- Status is recorded per-tool regardless of success/failure/timeout/missing

### No || true wrapping
- Task is not wrapped in `|| true` or `set +e` for the whole task
- Each tool's exit is explicitly captured and handled individually

### Document owner waiver
- Task includes comment referencing the 2026-05-23 owner waiver
- Comment explains audit remains fail-closed while deny/vet/geiger/machete are advisory

## Implementation Plan

### Per-tool report runner
```bash
run_report() {
    # Args: tool_name, command...
    # Returns: 0 always (advisory), captures status in file
    local tool="$1"; shift
    local out_file="target/supply-chain/${tool}.out"
    local err_file="target/supply-chain/${tool}.err"
    local status_file="target/supply-chain/${tool}.status"

    printf '%s\n' "Running ${tool}..." >&2
    "$@" >"$out_file" 2>"$err_file"
    local rc=$?
    printf '%d\n' "$rc" >"$status_file"
    return 0  # advisory: always continue
}
```

### Audit remains fail-closed
```bash
# Audit: actual vulnerabilities = fail
if ! timeout 2m rustup run nightly-2026-04-28 cargo audit --quiet \
     >target/supply-chain/audit.out 2>target/supply-chain/audit.err; then
    local rc=$?
    printf '%d\n' "$rc" >target/supply-chain/audit.status
    echo "cargo audit found vulnerabilities (exit $rc)" >&2
    cat target/supply-chain/audit.out target/supply-chain/audit.err >&2
    return 1
fi
printf '0\n' >target/supply-chain/audit.status
```

### Advisory tool invocation
```bash
# Deny: advisory report only
run_report deny timeout 5m rustup run nightly-2026-04-28 cargo deny check --hide-inclusion-graph

# Vet: advisory report only
run_report vet timeout 5m rustup run nightly-2026-04-28 cargo vet --store-path supply-chain --locked --verbose error

# Geiger: per-package, advisory report only
for package in ...; do
    run_report "geiger-${package}" timeout 90s bash -c "cd crates/${package} && rustup run nightly-2026-04-28 cargo geiger ..."
done

# Machete: advisory report only
run_report machete timeout 2m rustup run nightly-2026-04-28 cargo machete ...
```

## Verification
- Run `moon run supply-chain` on the isolated workspace
- Verify audit finds RUSTSEC-2024-0449 (unsafe-libyaml) and fails
- Verify deny/vet/geiger/machete capture output and continue even if nonzero
- Verify status files exist with exact exit codes
- Verify no `|| true` wrapping the entire task

## Owner Waiver Reference
The 2026-05-23 owner waiver (documented in MASTER lines 68, 114, 131, 149-153, 173, 229-232, 1683, 1711-1715, 1916, 4458, 4786) explicitly makes deny/vet/geiger/machete advisory reports. Only audit (actual vulnerabilities) and first-party unsafe lint enforcement remain blocking. This task does NOT add new waivers or suppress findings; it conforms the task to existing policy.

## Constraints
- No new shell scripts (bounded to `.moon/tasks/all.yml`)
- Keep geiger per-package invocation unchanged
- Preserve existing geiger workaround for `Dependency file was never scanned` warnings
- Do not alter first-party unsafe lint enforcement

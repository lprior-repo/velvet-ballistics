## Moon Gate Results

| Gate | Exit Code | Result |
|------|-----------|--------|
| :test | 1 | FAIL |
| :ci | 1 | NOT DEFINED |

### Details

**:test** - Failed at `supply-chain` task. Vetting failed due to 3 unvetted dependencies:
- `ryu:1.0.23` missing ["safe-to-deploy"]
- `serde_yaml:0.9.34+deprecated` missing ["safe-to-deploy"]
- `unsafe-libyaml:0.2.11` missing ["safe-to-deploy"]

**:ci** - No tasks found. Task pipeline not defined for this workspace.

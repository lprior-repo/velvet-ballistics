## Moon Gate Results

| Gate | Exit Code | Result |
|------|-----------|--------|
| :test | 255 | FAIL |
| :ci | N/A | NOT DEFINED |

### Notes

- **:test**: Failed at `supply-chain` task (exit 255). Vetting failed due to 3 unvetted dependencies: `ryu`, `serde_yaml`, `unsafe-libyaml` missing "safe-to-deploy" annotation.
- **:ci**: No tasks found. The `:ci` target is not defined in moon.yml.

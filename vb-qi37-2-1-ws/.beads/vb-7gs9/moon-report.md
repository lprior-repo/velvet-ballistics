## Moon Gate Results

| Gate | Exit Code | Result |
|------|-----------|--------|
| :test | 1 | FAIL |
| :ci | 1 | NOT DEFINED |

### Details

**:test (FAIL)**
- Supply-chain vet failed: 3 unvetted dependencies (`ryu`, `serde_yaml`, `unsafe-libyaml`)
- Advisory database fetched successfully (1068 security advisories)
- Process exited with code 255

**:ci (NOT DEFINED)**
- Task `:ci` does not exist in the bead workspace moon configuration

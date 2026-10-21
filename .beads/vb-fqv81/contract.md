# Contract: Dolt Metadata Port Separation

## Objective
Remove `dolt_server_port` from the Dolt metadata file. Port is tracked exclusively in bead `.beads/dolt-server.port`.

## Actual Metadata (before)
```json
{
  "database": "dolt",
  "backend": "dolt",
  "dolt_mode": "server",
  "dolt_server_host": "127.0.0.1",
  "dolt_server_port": 44101,
  "dolt_database": "velvet-ballistics",
  "project_id": "3265bb22-ec7c-4f87-b1a5-6001b941b612"
}
```

## Actual Metadata (after)
```json
{
  "database": "dolt",
  "backend": "dolt",
  "dolt_mode": "server",
  "dolt_server_host": "127.0.0.1",
  "dolt_database": "velvet-ballistics",
  "project_id": "3265bb22-ec7c-4f87-b1a5-6001b941b612"
}
```

## Bead Evidence
- `.beads/dolt-server.port` contains `44101` (unchanged)
- Active healthy bd-managed server at `127.0.0.1:44101`
- Worktree bd routing: main

## Constraints
- No server restart
- No server delete
- No database rename
- No remotes changes
- Scope: metadata file + bead evidence/tests only

## Workflow
1. Rust contract
2. Implementation
3. Tests
4. Code review
5. Test review

# vb-18fhg — DeepSeek OMP Agent Roles Smoke Evidence

Bounded, benign, read-only smoke: parent OMP (v18.12.1) dispatched the three named
roles via the `task` tool in one batch; each role read the same random-token fixture
and reported it back. Projectless run in `/tmp/vb-omp-smoke` (outside the repo); no
repo file was edited by any child.

Fixture token (random, generated for this smoke, not a secret): `vb18fhg-bf7eb0eca00112f64e9b02ab`

## Discovery

All three roles were discovered from the user agent source (`~/.omp/agent/agents/`)
and completed with `exitCode 0`. No "Unknown agent" preflight failure.

## Per-role resolved model evidence

| role | resolvedModel | resolvedModelIdentity | resolvedThinkingLevel | resolvedModelIsFallback | agentSource |
| --- | --- | --- | --- | --- | --- |
| vb-deepseek-rust | `deepseek/deepseek-v4-pro:high` | `deepseek/deepseek-v4-pro` | `high` | `false` | user |
| vb-deepseek-code-review | `deepseek/deepseek-v4-pro:high` | `deepseek/deepseek-v4-pro` | `high` | `false` | user |
| vb-deepseek-test-review | `deepseek/deepseek-v4-pro:high` | `deepseek/deepseek-v4-pro` | `high` | `false` | user |

`resolvedModelIsFallback: false` confirms the model came from the agent frontmatter
`model: [deepseek/deepseek-v4-pro]` (not a parent/fallback model).

## Tool execution evidence

Every role executed the `read` tool against the fixture and returned the exact token
in its final answer:

- vb-deepseek-rust: `read /tmp/vb-omp-smoke/token.txt` (plus read-only `bash` `od`/`xxd`
  to confirm exact bytes) -> reported token verbatim, 32 chars, no trailing newline.
- vb-deepseek-code-review: `read /tmp/vb-omp-smoke/token.txt` -> reported token verbatim.
- vb-deepseek-test-review: `read /tmp/vb-omp-smoke/token.txt` -> reported token verbatim.

All three final answers matched the fixture token exactly (no hardcoded response).

## Parent pin

Parent smoke run used `--model deepseek/deepseek-v4-pro --smol deepseek/deepseek-v4-pro
--slow deepseek/deepseek-v4-pro --plan deepseek/deepseek-v4-pro --thinking high`, with
`async.enabled: false` so the batch blocked and returned settled results.

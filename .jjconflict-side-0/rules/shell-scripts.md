# Shell scripts

Read this when writing or editing a shell script — bash/sh or PowerShell — anything beyond a single command.

- Bash starts with `set -euo pipefail`. Handle the known escape hatches (`|| true`, `if cmd; then`) where a non-zero exit is legitimate, instead of dropping the flags for the whole script.
- Quote every expansion: `"$var"`, `"$@"`, `"$(cmd)"`. Unquoted expansion is word-splitting and globbing — the classic path-with-spaces bug.
- Build argument lists in arrays, not concatenated strings, which reintroduces the same splitting bug.
- Temp files come from `mktemp` and die in `trap 'rm -rf "$tmp"' EXIT` — cleanup that also runs on error and interrupt, not just the happy path.
- A script with options gets `getopts`, a `usage()` printed on `-h` and on bad input, and a non-zero exit on misuse; exit codes are the script's API.
- `printf` over `echo` for data (`echo` mangles `-n` and backslashes, and varies by shell); `command -v` over `which`; `[[ ]]` over `[ ]` in bash — and if the shebang says `#!/bin/sh`, no bashisms at all.
- Run ShellCheck on every script; a suppressed finding carries its per-line `# shellcheck disable=SCnnnn` with the reason beside it.
- A script that mutates shared state is idempotent on re-run (guard completed steps), and grows a `--dry-run` flag once it deletes or provisions; the core Boundaries backup rule applies to scripts the same as to typed commands.
- A script runs where nobody answers a prompt: every command in it gets its non-interactive flag, and a command that still prompts fails the script. Repeated from the core Scope rule because a prompt in CI hangs until the job timeout instead of failing.
- A script whose output a later step reads or diffs produces the same bytes on unchanged input: sorted ordering, no timestamps or absolute paths (`rules/api-contracts.md` for generated code).
- PowerShell starts with `Set-StrictMode -Version Latest` and `$ErrorActionPreference = 'Stop'` — and still checks `$LASTEXITCODE` after native executables, which `Stop` does not cover.
- PowerShell scripts use full cmdlet names (no `ls`/`%`/`?` aliases) and a typed `param()` block over positional `$args`; destructive cmdlets get `-Confirm:$false` only when the script has already obtained its confirmation.
- No credentials inline — reference `$env:`/a secret manager (core Security rule); generated scripts get the same review and verification gate as any other code.

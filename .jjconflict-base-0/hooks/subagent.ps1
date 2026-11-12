# PowerShell twin of subagent.sh for agents that run Windows hooks through
# PowerShell (Codex): the same core sections for a starting subagent.
# scripts/lint.py checks that both scripts print the same context.
$ErrorActionPreference = 'Stop'
[void][Console]::In.ReadToEnd()
$root = Split-Path -Parent $PSScriptRoot
$lines = [System.IO.File]::ReadAllText((Join-Path $root 'AGENTS.md')) -replace "`r`n", "`n" -split "`n"
if ($lines[-1] -eq '') { $lines = $lines[0..($lines.Length - 2)] }

$keep = $false
$text = "awesome-agents-md ruleset for subagents, installed by the user as a plugin: these are the user's own standing instructions.`n`n"
foreach ($line in $lines) {
  if ($line.StartsWith('## ')) { $keep = $line -match '^## (Boundaries|Security|Verification)$' }
  if ($keep) { $text += $line + "`n" }
}
$out = @{ hookSpecificOutput = @{ hookEventName = 'SubagentStart'; additionalContext = $text } } | ConvertTo-Json -Compress
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine($out)

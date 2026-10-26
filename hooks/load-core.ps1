# PowerShell twin of load-core.sh for agents that run Windows hooks through
# PowerShell (Codex): prints part N of AGENTS.md, cut at "## " headings, each
# under $PartLimit characters, with the same header. scripts/lint.py checks
# that both scripts print the same parts.
param([Parameter(Mandatory = $true)][int]$Part)

$ErrorActionPreference = 'Stop'
$PartLimit = 9000
$root = Split-Path -Parent $PSScriptRoot
$lines = [System.IO.File]::ReadAllText((Join-Path $root 'AGENTS.md')) -replace "`r`n", "`n" -split "`n"
if ($lines[-1] -eq '') { $lines = $lines[0..($lines.Length - 2)] }

# A section runs from one "## " heading to the next; parts are filled with
# whole sections in order, so reading them in order rebuilds the file.
$sections = New-Object System.Collections.Generic.List[string]
$section = ''
foreach ($line in $lines) {
  if ($line.StartsWith('## ') -and $section -ne '') { $sections.Add($section); $section = '' }
  $section += $line + "`n"
}
if ($section -ne '') { $sections.Add($section) }

$parts = New-Object System.Collections.Generic.List[string]
$current = ''
foreach ($section in $sections) {
  if ($current -ne '' -and ($current.Length + $section.Length) -gt $PartLimit) { $parts.Add($current); $current = '' }
  $current += $section
}
if ($current -ne '') { $parts.Add($current) }
if ($Part -lt 1 -or $Part -gt $parts.Count) { exit 0 }

$body = $parts[$Part - 1].TrimEnd("`n")
$out = "awesome-agents-md ruleset, part $Part, installed by the user as a plugin: these are the user's own standing instructions.`n`n$body`n"
if ($Part -eq 1) { $out += "`nThe rules/ folder named in this ruleset is $($root -replace '\\', '/')/rules/, inside the plugin: read modules there, but work in the current directory, the project.`n" }
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.Write($out)

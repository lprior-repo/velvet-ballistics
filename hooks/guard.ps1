# PowerShell twin of guard.sh for agents that run Windows hooks through
# PowerShell (Codex): the same rule tables, the same messages, exit 2 to block.
# scripts/lint.py runs both scripts against the same commands.
$ErrorActionPreference = 'Stop'

$raw = [Console]::In.ReadToEnd()
try {
  $payload = $raw | ConvertFrom-Json
} catch {
  # A payload that names a command the guard cannot read (cut off, or in a
  # shape it does not parse) goes to the user rather than through unchecked.
  if ($raw -match '"command"\s*:') {
    [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"awesome-agents-md: the guard could not read this command. Approve only if you have read it."}}')
  }
  exit 0
}
$toolInput = $payload.tool_input
if ($toolInput -is [string]) { $toolInput = $toolInput | ConvertFrom-Json }
$command = [string]$toolInput.command
if (-not $command) { exit 0 }
# A newline separates commands the way ; does. Known ceiling: a command
# assembled at run time (a variable, a script file, git commit -F) is not seen.
$command = $command -replace "`r?`n", '; '

function Test-Command([string]$pattern) {
  return [regex]::IsMatch($command, $pattern, 'IgnoreCase')
}

# A segment starts at the line start or after ; & | ( or $( and runs to the
# next separator, so [^;&|]* keeps a flag tied to the git call it belongs to.
$start = '(^|[;&|(]|\$\()\s*'
$secret = '[A-Za-z0-9_]*(KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL)[A-Za-z0-9_]*'

# Block rules: pattern, exception pattern (a match lets the command through),
# message.
$blockRules = @(
  # Printing the environment. A listing reduced to variable names prints no
  # value, and it is how an agent finds a missing variable, so it goes through.
  @{ Pattern = "(^|[^A-Za-z0-9_-])printenv([^A-Za-z0-9_-]|$)|$start(env|set)\s*($|[;&|)])|declare\s+-[A-Za-z0-9]*p|/proc/\S*/environ|(Get-ChildItem|gci|dir|ls|Get-Item|gi)\s+(-Path\s+)?env:(\*|[\s;|]|$)"
     Unless = '[|]\s*(select(-object)?\s+(-expandproperty\s+)?name|%\s*[{]\s*[$]_\.name|cut\s+-d\s*.?=.?\s*-f\s*1)\s*($|[|;&)])'
     Message = 'printing the environment. Its output enters the transcript with every credential in it: read the one value the task needs, or send both streams to the null device.' },
  # Printing one variable whose name marks a secret: a bare PowerShell $env:X
  # statement, or echo/printf/Write-* of it. Testing that it is set, or
  # passing it to a command, prints nothing.
  @{ Pattern = "$start\`$env:$secret\s*($|[;|)])|(echo|printf|print|Write-Host|Write-Output)\s[^;&|]*\`$(env:)?\{?$secret"
     Unless = ''
     Message = 'printing a secret variable. Its value enters the transcript: test that it is set ([ -n "$X" ], Test-Path env:X) instead of printing it.' },
  # A recursive delete of the root, home, working directory or .git.
  # rm -rf build stays ordinary work.
  @{ Pattern = $start + '(sudo\s+)?(rm|Remove-Item)\s([^;&|]*\s)?(-[dfirv]*r[dfirv]*|--recursive|-rec[A-Za-z]*)\s([^;&|]*\s)?(/|/\*|~/?|\.{1,2}/?|\*|\.git/?|\$HOME/?)(\s|$|[;&|)])'
     Unless = ''
     Message = 'recursively deleting the root, the home or working directory, or .git.' },
  @{ Pattern = 'git\s[^;&|]*(reset[^;&|]*\s--hard|clean[^;&|]*\s(-[A-Za-z0-9]*f|--force))'
     Unless = ''
     Message = 'discarding uncommitted work for good (git reset --hard, git clean -f).' },
  @{ Pattern = 'git\s[^;&|]*(--no-verify|core\.hooksPath)'
     Unless = ''
     Message = 'skipping git hooks. A failing hook is a check: fix what it reports.' },
  @{ Pattern = '(drop\s+(table|database|schema)|truncate\s+(table\s+)?[A-Za-z_])'
     Unless = ''
     Message = 'dropping or emptying a table or database.' },
  @{ Pattern = 'delete\s+from\s'
     Unless = 'delete\s+from\s[^;]*\swhere\s'
     Message = 'deleting every row of a table (DELETE without WHERE).' },
  @{ Pattern = '(curl|wget|iwr|irm|Invoke-WebRequest|Invoke-RestMethod)\s[^;&|]*[|]\s*(sudo\s+)?((ba|z|da)?sh|iex|Invoke-Expression|pwsh|powershell)(\s|$|[;&|)])|(iex|Invoke-Expression)\s*[(]\s*(irm|iwr|Invoke-RestMethod|Invoke-WebRequest)|(^|[^A-Za-z0-9_-])(ba|z)?sh\s+(-c\s+)?\S{0,3}(<|[$])[(]\s*(curl|wget)'
     Unless = ''
     Message = 'running a script downloaded from the network without reading it.' }
)

$issues = @()
foreach ($rule in $blockRules) {
  if ((Test-Command $rule.Pattern) -and (-not $rule.Unless -or -not (Test-Command $rule.Unless))) {
    $issues += $rule.Message
  }
}
if ($issues.Count -gt 0) {
  foreach ($issue in $issues) { [Console]::Error.WriteLine("Blocked by the awesome-agents-md guard: $issue") }
  exit 2
}

# An AI coding agent named as co-author: a GitHub app ([bot]), the address an
# agent signs with, or the agent's name. A human co-author passes. Sources:
# the agent-dora and CHAOSS disclosure detection lists and the tools' own
# commits; an entry is an address or a product name, never a company domain
# a person could write from.
$agent = '\[bot\]|@(anthropic\.com|openai\.com|cursor\.com|aider\.chat|all-hands\.dev|ampcode\.com|charm\.land|factory\.ai|opencode\.ai|windsurf\.(ai|com)|continue\.dev|kiro\.dev|cline\.bot|roocode\.com|kilocode\.ai|kilo\.ai|block\.xyz|aaif\.ai|augmentcode\.com|warp\.dev|zed\.dev|air\.dev|mistral\.ai|charlielabs\.ai|x\.ai|muse\.dev)|(copilot@github|gemini-code-assist@google|junie@jetbrains|qwen-coder@alibabacloud|it\+v0agent@vercel|noreply@meta)\.com|[0-9]+\+copilot@|(^|[^A-Za-z0-9])(claude|anthropic|copilot|codex|chatgpt|openai|cursor|gemini|jules|devin|aider|openhands|amp|crush|opencode|droid|windsurf|cascade|kiro|amazon-q|cline|roo code|roomote|kilo code|goose|augment|junie|mistral vibe|qwen-coder|v0agent|lovable|gpt-engineer|codegen|grok|muse code|coderabbit|mentat|sweep|greptile|cody|codium|replit)([^A-Za-z0-9]|$)'

# Ask rules: pattern, second pattern that must match too (a trailer sits in
# the message body, past the segment the commit call is in), message. Whether
# a prompt asked for the push or the trailer cannot be read from its words in
# every language, so the user confirms the call itself.
$askRules = @(
  @{ Pattern = 'git\s[^;&|]*commit'
     Also = "co-authored-by\s*:[^;]*($agent)"
     Message = 'this commit names an AI agent as co-author. Commits carry no assistant trace: approve only if you asked for it.' },
  @{ Pattern = 'git\s[^;&|]*push(\s|$)'
     Also = ''
     Message = 'this pushes to a remote. Approve only if you asked for it.' }
)
foreach ($rule in $askRules) {
  if ((Test-Command $rule.Pattern) -and (-not $rule.Also -or (Test-Command $rule.Also))) {
    [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"awesome-agents-md: ' + $rule.Message + '"}}')
    exit 0
  }
}
exit 0

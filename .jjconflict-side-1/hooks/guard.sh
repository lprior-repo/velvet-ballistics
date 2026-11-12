#!/usr/bin/env bash
# PreToolUse guard for shell tools, in the shape of Anthropic's
# bash_command_validator example: a table of (pattern, message) rules run over
# the command; every matching block rule is reported and exit 2 blocks the
# call, with stderr handed to the agent as the reason. Two more rules ask the
# user instead: a push, and a commit naming an AI agent as co-author.
# Known ceiling: the patterns see the command text, not its syntax, so a
# forbidden word inside a quoted argument matches too.
set -euo pipefail

payload=$(cat)

# Only git and the OS shell are assumed, so the command field is pulled out with
# a regex instead of a JSON parser. JSON escapes stay in the text; an escaped
# newline becomes a command separator. Known ceiling: a command assembled at run
# time (a variable, a script file, git commit -F) is not seen.
command=$(printf '%s' "$payload" |
  grep -oE '"command"[[:space:]]*:[[:space:]]*"([^"\\]|\\.)*"' |
  head -n 1 |
  sed -E 's/^"command"[[:space:]]*:[[:space:]]*"//; s/"$//; s/\\n/; /g') || true
if [ -z "$command" ]; then
  # A payload that names a command the guard cannot read (cut off, or in a
  # shape it does not parse) goes to the user rather than through unchecked.
  if printf '%s' "$payload" | grep -qE '"command"[[:space:]]*:'; then
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"awesome-agents-md: the guard could not read this command. Approve only if you have read it."}}'
  fi
  exit 0
fi

matches() {
  printf '%s' "$command" | grep -qEi -- "$1"
}

# A segment starts at the line start or after ; & | ( or $( and runs to the
# next separator, so [^;&|]* keeps a flag tied to the git call it belongs to.
start='(^|[;&|(]|\$\()[[:space:]]*'
secret='[[:alnum:]_]*(KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL)[[:alnum:]_]*'

# Block rules, three fields each: pattern, exception pattern (a match lets the
# command through), message.
block_rules=(
  # Printing the environment. A listing reduced to variable names prints no
  # value, and it is how an agent finds a missing variable, so it goes through.
  "(^|[^[:alnum:]_-])printenv([^[:alnum:]_-]|$)|${start}(env|set)[[:space:]]*($|[;&|)])|declare[[:space:]]+-[[:alnum:]]*p|/proc/[^[:space:]]*/environ|(Get-ChildItem|gci|dir|ls|Get-Item|gi)[[:space:]]+(-Path[[:space:]]+)?env:(\*|[[:space:];|]|$)"
  '[|][[:space:]]*(select(-object)?[[:space:]]+(-expandproperty[[:space:]]+)?name|%[[:space:]]*[{][[:space:]]*[$]_\.name|cut[[:space:]]+-d[[:space:]]*.?=.?[[:space:]]*-f[[:space:]]*1)[[:space:]]*($|[|;&)])'
  'printing the environment. Its output enters the transcript with every credential in it: read the one value the task needs, or send both streams to the null device.'
  # Printing one variable whose name marks a secret: a bare PowerShell $env:X
  # statement, or echo/printf/Write-* of it. Testing that it is set, or
  # passing it to a command, prints nothing.
  "${start}\\\$env:${secret}[[:space:]]*(\$|[;|)])|(echo|printf|print|Write-Host|Write-Output)[[:space:]][^;&|]*\\\$(env:)?\\{?${secret}"
  ''
  'printing a secret variable. Its value enters the transcript: test that it is set ([ -n "$X" ], Test-Path env:X) instead of printing it.'
  # A recursive delete of the root, home, working directory or .git.
  # rm -rf build stays ordinary work.
  "${start}"'(sudo[[:space:]]+)?(rm|Remove-Item)[[:space:]]([^;&|]*[[:space:]])?(-[dfirv]*r[dfirv]*|--recursive|-rec[[:alpha:]]*)[[:space:]]([^;&|]*[[:space:]])?(/|/\*|~/?|\.{1,2}/?|\*|\.git/?|\$HOME/?)([[:space:]]|$|[;&|)])'
  ''
  'recursively deleting the root, the home or working directory, or .git.'
  'git[[:space:]][^;&|]*(reset[^;&|]*[[:space:]]--hard|clean[^;&|]*[[:space:]](-[[:alnum:]]*f|--force))'
  ''
  'discarding uncommitted work for good (git reset --hard, git clean -f).'
  'git[[:space:]][^;&|]*(--no-verify|core\.hooksPath)'
  ''
  'skipping git hooks. A failing hook is a check: fix what it reports.'
  '(drop[[:space:]]+(table|database|schema)|truncate[[:space:]]+(table[[:space:]]+)?[[:alpha:]_])'
  ''
  'dropping or emptying a table or database.'
  'delete[[:space:]]+from[[:space:]]'
  'delete[[:space:]]+from[[:space:]][^;]*[[:space:]]where[[:space:]]'
  'deleting every row of a table (DELETE without WHERE).'
  '(curl|wget|iwr|irm|Invoke-WebRequest|Invoke-RestMethod)[[:space:]][^;&|]*[|][[:space:]]*(sudo[[:space:]]+)?((ba|z|da)?sh|iex|Invoke-Expression|pwsh|powershell)([[:space:]]|$|[;&|)])|(iex|Invoke-Expression)[[:space:]]*[(][[:space:]]*(irm|iwr|Invoke-RestMethod|Invoke-WebRequest)|(^|[^[:alnum:]_-])(ba|z)?sh[[:space:]]+(-c[[:space:]]+)?[^[:space:]]{0,3}(<|[$])[(][[:space:]]*(curl|wget)'
  ''
  'running a script downloaded from the network without reading it.'
)

issues=()
for ((i = 0; i < ${#block_rules[@]}; i += 3)); do
  if matches "${block_rules[i]}" && { [ -z "${block_rules[i + 1]}" ] || ! matches "${block_rules[i + 1]}"; }; then
    issues+=("${block_rules[i + 2]}")
  fi
done
if [ ${#issues[@]} -gt 0 ]; then
  printf 'Blocked by the awesome-agents-md guard: %s\n' "${issues[@]}" >&2
  exit 2
fi

# An AI coding agent named as co-author: a GitHub app ([bot]), the address an
# agent signs with, or the agent's name. A human co-author passes. Sources:
# the agent-dora and CHAOSS disclosure detection lists and the tools' own
# commits; an entry is an address or a product name, never a company domain
# a person could write from.
agent='\[bot\]|@(anthropic\.com|openai\.com|cursor\.com|aider\.chat|all-hands\.dev|ampcode\.com|charm\.land|factory\.ai|opencode\.ai|windsurf\.(ai|com)|continue\.dev|kiro\.dev|cline\.bot|roocode\.com|kilocode\.ai|kilo\.ai|block\.xyz|aaif\.ai|augmentcode\.com|warp\.dev|zed\.dev|air\.dev|mistral\.ai|charlielabs\.ai|x\.ai|muse\.dev)|(copilot@github|gemini-code-assist@google|junie@jetbrains|qwen-coder@alibabacloud|it\+v0agent@vercel|noreply@meta)\.com|[0-9]+\+copilot@|(^|[^A-Za-z0-9])(claude|anthropic|copilot|codex|chatgpt|openai|cursor|gemini|jules|devin|aider|openhands|amp|crush|opencode|droid|windsurf|cascade|kiro|amazon-q|cline|roo code|roomote|kilo code|goose|augment|junie|mistral vibe|qwen-coder|v0agent|lovable|gpt-engineer|codegen|grok|muse code|coderabbit|mentat|sweep|greptile|cody|codium|replit)([^A-Za-z0-9]|$)'

# Ask rules, three fields each: pattern, second pattern that must match too
# (a trailer sits in the message body, past the segment the commit call is
# in), message. Whether a prompt asked for the push or the trailer cannot be
# read from its words in every language, so the user confirms the call itself.
ask_rules=(
  'git[[:space:]][^;&|]*commit'
  "co-authored-by[[:space:]]*:[^;]*(${agent})"
  'this commit names an AI agent as co-author. Commits carry no assistant trace: approve only if you asked for it.'
  'git[[:space:]][^;&|]*push([[:space:]]|$)'
  ''
  'this pushes to a remote. Approve only if you asked for it.'
)
for ((i = 0; i < ${#ask_rules[@]}; i += 3)); do
  if matches "${ask_rules[i]}" && { [ -z "${ask_rules[i + 1]}" ] || matches "${ask_rules[i + 1]}"; }; then
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"awesome-agents-md: %s"}}\n' "${ask_rules[i + 2]}"
    exit 0
  fi
done
exit 0

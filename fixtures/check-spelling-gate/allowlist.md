# Allowlist Fixture — every content pattern is exercised

Each line below includes the forbidden spelling only inside one documented
content-exclusion pattern. The gate must report zero violations for this file
when it is scanned from a non-excluded path.

Pattern 1 master-file reference: velvet-ballistics-MASTER.md

Pattern 2 source-checkout path: /home/lewis/src/velvet-ballistics/

Pattern 3 forbid-tag line: FORBIDDEN_FEATURE_NAMES blocks velvet-ballistics.

Pattern 4 rule statement: `velvet-ballistics` is invalid except in migration prose.

Pattern 5 Dolt remote URL: https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics

Pattern 6 legacy version string: velvet-ballistics/v2

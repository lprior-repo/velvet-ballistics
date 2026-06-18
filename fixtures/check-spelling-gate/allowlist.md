# Allowlist Fixture — every content pattern is exercised

Each matching line below includes the forbidden spelling only inside one documented
content-exclusion pattern. The gate must report zero violations for this file
when it is scanned from a non-excluded path.

Pattern 1 master-file reference: velvet-ballistics-MASTER.md

Pattern 2 source-checkout path: /home/lewis/src/velvet-ballistics/

Pattern 3 legacy-name documentation: Legacy names such as `Velvet Ballastics`, `velvet`, and `vb` are not valid for new docs.

Pattern 4 ADR regex documentation: rg -n "Velvet Ballastics|Velvet-ballistics|vb-core" docs

Pattern 5 Dolt remote URL: https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics

Pattern 6 legacy version string: velvet-ballistics/v2

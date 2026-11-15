const fs = require("node:fs");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const base = "f366003714753937573e04650bf63cc4e5beb8ea";
const names = cp.execFileSync("git", ["diff", "--name-only", base, "--", "crates", "verification"], {encoding: "utf8"}).trim().split("\n").filter(Boolean);
const digest = bytes => crypto.createHash("sha256").update(bytes).digest("hex");
const files = names.map(path => ({path, sha256: digest(fs.readFileSync(path))}));
const bindings = [
  ".evidence/taxonomy60/retained-v3.sha256",
  ".evidence/taxonomy60/retry-retained-v1.sha256",
  ".evidence/taxonomy60/identity-owner-manifest.json",
  ".evidence/taxonomy60/unsafe-retry-CODE-review-amended.md.gz",
  ".evidence/taxonomy60/unsafe-retry-Verus-review-amended.md.gz",
  ".evidence/taxonomy60/retained-fixture-CODE-review.md.gz",
  ".evidence/taxonomy60/test-suite-review.md",
  ".evidence/taxonomy60/followup-test-suite-review.md",
].map(path => ({path, sha256: digest(fs.readFileSync(path))}));
const report = {
  base,
  canonical_status: "BLOCKED: exhaustive MASTER65 implications do not admit the actual local Fjall executor",
  cold_adapter_implemented: false,
  source_count: files.length,
  source: files,
  bindings,
  parent_observed_native_tests: {combined: 47, existing_helper: 29, failures: 0},
  parent_observed_verus: {verified: 34, errors: 0, strong_bindings: 0, weak_bindings: 34, vacuum_bindings: 0, native_proof_closed: false},
  archived_files: {original: 284, retry: 131, total: 415, all_unchanged: true},
  identity_owner_sources: {count: 25, all_unchanged: true},
  joined_ci_owner: "central integration",
};
fs.writeFileSync(".evidence/taxonomy60/source-manifest.json", JSON.stringify(report, null, 2) + "\n");
process.stdout.write(JSON.stringify({source_count: report.source_count, source: files}) + "\n");

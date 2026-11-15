const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const cp = require("node:child_process");

const root = process.cwd();
const base = "f366003714753937573e04650bf63cc4e5beb8ea";
const ownerFiles = [
  "crates/vb_storage/src/executor/catalog.rs",
  "crates/vb_storage/src/admission/verification.rs",
];
const entries = new Map();
for (const owner of ownerFiles) {
  const source = fs.readFileSync(path.join(root, owner), "utf8");
  const inputs = [...source.matchAll(/include_bytes!\(\s*"([^"]+)"\s*\)/g)];
  for (const input of inputs) {
    const absolute = path.resolve(root, path.dirname(owner), input[1]);
    const relative = path.relative(root, absolute);
    const owners = entries.get(relative) ?? [];
    owners.push(owner);
    entries.set(relative, owners);
  }
}
for (const record of [
  "crates/vb_core/src/action/contract.rs",
  "crates/vb_runtime/src/run_checkpoint.rs",
]) {
  entries.set(record, ["retained wire record"]);
}
const sha256 = bytes => crypto.createHash("sha256").update(bytes).digest("hex");
const files = [...entries.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([relative, owners]) => {
  const current = fs.readFileSync(path.join(root, relative));
  const original = cp.execFileSync("git", ["show", base + ":" + relative], {cwd: root});
  return {path: relative, owners, sha256: sha256(current), base_sha256: sha256(original), unchanged: current.equals(original)};
});
const report = {base, files, count: files.length, all_unchanged: files.every(file => file.unchanged)};
fs.writeFileSync(path.join(root, ".evidence/taxonomy60/identity-owner-manifest.json"), JSON.stringify(report, null, 2) + "\n");
process.stdout.write(JSON.stringify({count: report.count, all_unchanged: report.all_unchanged}) + "\n");
if (!report.all_unchanged) process.exitCode = 1;

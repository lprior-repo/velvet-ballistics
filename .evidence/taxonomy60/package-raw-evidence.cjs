const fs = require("node:fs");
const cp = require("node:child_process");
const zlib = require("node:zlib");
const identity = ["user.name", "user.email"].flatMap(key => [
  "--config", key + "=" + JSON.stringify(cp.execFileSync("git", ["config", "--get", key], {encoding: "utf8"}).trim()),
]);
let output = "";
try {
  output = cp.execFileSync("git", ["diff", "--check"], {encoding: "utf8"});
} catch (error) {
  if (error.status !== 2 && error.status !== 1) throw error;
  output = String(error.stdout);
}
const files = [...new Set([...output.matchAll(/^(.+?):\d+:/gm)].map(match => match[1]))];
for (const file of files) {
  if (!file.startsWith(".evidence/taxonomy60/")) throw new Error("Source whitespace finding: " + file);
  const original = fs.readFileSync(file);
  const compressed = file + ".gz";
  fs.writeFileSync(compressed, zlib.gzipSync(original));
  if (!zlib.gunzipSync(fs.readFileSync(compressed)).equals(original)) throw new Error("Evidence mismatch");
  cp.execFileSync("jj", [...identity, "file", "untrack", file]);
  cp.execFileSync("jj", [...identity, "file", "track", "--include-ignored", compressed]);
}
process.stdout.write(JSON.stringify({packaged_exact_raw_files: files.length}) + "\n");

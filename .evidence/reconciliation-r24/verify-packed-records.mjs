import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../..');
const manifest = JSON.parse(fs.readFileSync(path.join(directory, 'packed-raw-records.json')));
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const expected = new Set([".evidence/reconciliation-r24/repairs/trace78-code.md",".evidence/reconciliation-r24/reviews/code-transport.md",".evidence/reconciliation-r24/verification/trace78-baseline.log",".evidence/reconciliation-r24/verification/trace78-baseline2.log",".evidence/reconciliation-r24/verification/trace78-miri.log"]);
assert.equal(manifest.records.length, expected.size);
assert.deepEqual(new Set(manifest.records.map(record => record.original_path)), expected);
for (const record of manifest.records) {
  assert(record.original_path.startsWith('.evidence/reconciliation-r24/'));
  assert.equal(record.transport_path, record.original_path + '.gz');
  const compressed = fs.readFileSync(path.join(root, record.transport_path));
  assert.equal(digest(compressed), record.transport_sha256);
  assert.equal(compressed.length, record.transport_bytes);
  const restored = zlib.gunzipSync(compressed);
  assert.equal(digest(restored), record.original_sha256);
  assert.equal(restored.length, record.original_bytes);
  const original = path.join(root, record.original_path);
  if (fs.existsSync(original)) assert.deepEqual(restored, fs.readFileSync(original));
}
console.log('5/5 gzip records restore exact original bytes and SHA-256.');

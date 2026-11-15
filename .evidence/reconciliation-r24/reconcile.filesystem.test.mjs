import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { regenerate } from './reconcile.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const tuple = { path: 'crates/domain/src/lib.rs', old: '0'.repeat(40), new: 'a'.repeat(40), status: 'A' };
const group = { id: 'source', tips: ['tip'], refs: ['branch', 'workspace'], changes: [tuple] };
const aliases = [{ index: 0, id: group.id, tips: group.tips, refs: group.refs }];
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

function entry(directory, name, value, format = 'jsonl') {
  const file = path.join(directory, name + '.' + format);
  const bytes = format === 'json' ? JSON.stringify(value) : value.map(row => JSON.stringify(row) + '\n').join('');
  fs.writeFileSync(file, bytes);
  return { name, file, format, sha256: digest(bytes) };
}

function fixture() {
  fs.mkdirSync(path.join(here, 'test-data'), { recursive: true });
  const directory = fs.mkdtempSync(path.join(here, 'test-data', 'fixture-'));
  const canonical = [entry(directory, 'groups', [group]), entry(directory, 'functional', {}, 'json'),
    ...['delegated', 'tokens', 'bodies', 'manual'].map(name => entry(directory, name, [])),
    entry(directory, 'historical_production', [tuple])];
  const pending = { intent: 'typed-wrapper', qualification_owner: 'historical-owner' };
  const raw = { ...tuple, resolution_r23: { per_intent: [pending] } };
  const assessment = { ...tuple, source_aliases: aliases, disposition: 'mixed', reason: 'specific qualification', evidence: ['source.rs'],
    pending_intents: [{ intent: pending.intent, reason: 'qualify actual producer', owner: 'active-owner', source_obligations: [
      { id: 'receipt.resolution_r23.per_intent[0]', value: pending },
    ], owner_rebindings: [{ source_owner: 'historical-owner', current_owner: 'active-owner', reason: 'current live owner', evidence: ['tracker'] }] }] };
  const manifest = { anchor: 'frozen-source', expected_baseline_tuples: 1, canonical,
    packets: [entry(directory, 'packet', [raw])], assessments: [entry(directory, 'assessment', [assessment])],
    owners: entry(directory, 'owners', [{ id: 'active-owner', status: 'in_progress' }], 'json') };
  fs.writeFileSync(path.join(directory, 'input-manifest.json'), JSON.stringify(manifest));
  return { directory, manifest, pending };
}

function generatedBytes(directory) {
  return Object.fromEntries(fs.readdirSync(path.join(directory, 'generated')).sort().map(name => [
    name, fs.readFileSync(path.join(directory, 'generated', name), 'utf8'),
  ]));
}

test('regeneration preserves exact aliases and pending state deterministically', () => {
  const { directory } = fixture();
  assert.deepEqual(regenerate(directory), {
    anchor: 'frozen-source', baseline_tuples: 1, baseline_production_tuples: 1, assessed_tuples: 1,
    assessed_with_pending_intents: 1, remaining_unreviewed_tuples: 0, remaining_unreviewed_by_lane: {},
    source_aliases_preserved: true, all_reconciliation_complete: false, engine_acceptance_claimed: false,
    proof_or_suite_execution_claimed: false,
  });
  const first = generatedBytes(directory);
  assert.deepEqual(JSON.parse(first['baseline.jsonl']).source_aliases, aliases);
  regenerate(directory);
  assert.deepEqual(generatedBytes(directory), first);
});

test('changed frozen input is rejected before generated evidence is written', () => {
  const { directory, manifest } = fixture();
  fs.appendFileSync(manifest.canonical[0].file, '\n');
  assert.throws(() => regenerate(directory), /changed input/);
  assert.equal(fs.existsSync(path.join(directory, 'generated')), false);
});

test('binder retains real qualification-owner obligations and explicit rebinding', () => {
  const { directory, manifest, pending } = fixture();
  const original = JSON.parse(fs.readFileSync(manifest.assessments[0].file));
  original.pending_intents[0].source_obligations = [];
  original.pending_intents[0].owner_rebindings = [];
  manifest.assessments[0] = entry(directory, 'assessment', [original]);
  fs.writeFileSync(path.join(directory, 'input-manifest.json'), JSON.stringify(manifest));
  for (const file of ['bind-pending.mjs', 'reconcile.mjs']) fs.copyFileSync(path.join(here, file), path.join(directory, file));
  const result = spawnSync('node', [path.join(directory, 'bind-pending.mjs')], { cwd: directory, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const bound = JSON.parse(fs.readFileSync(manifest.assessments[0].file));
  assert.deepEqual(bound.pending_intents[0].source_obligations, [{ id: 'receipt.resolution_r23.per_intent[0]', value: pending }]);
  assert.equal(bound.pending_intents[0].owner_rebindings[0].source_owner, 'historical-owner');
  assert.equal(bound.pending_intents[0].owner_rebindings[0].current_owner, 'active-owner');
  assert.equal(regenerate(directory).assessed_with_pending_intents, 1);
});

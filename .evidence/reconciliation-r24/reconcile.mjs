import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const identityFields = ['path', 'old', 'new', 'status'];
export function key(row) {
  for (const field of identityFields) assert.equal(typeof row[field], 'string', `missing ${field}`);
  assert.match(row.old, /^[a-f0-9]{40}$/);
  assert.match(row.new, /^[a-f0-9]{40}$/);
  assert.match(row.status, /^[A-Z][0-9]*$/);
  return JSON.stringify(identityFields.map(field => row[field]));
}

export function canonical(groups, functional, exactKnown) {
  const aliases = new Map();
  const rows = new Map();
  const known = new Set(exactKnown.map(key));
  groups.forEach((group, index) => {
    for (const row of group.changes) {
      const id = key(row);
      rows.set(id, Object.fromEntries(identityFields.map(field => [field, row[field]])));
      const sources = aliases.get(id) || [];
      sources.push({ index, id: group.id, tips: group.tips, refs: group.refs });
      aliases.set(id, sources);
      if (Object.hasOwn(functional, String(index))) known.add(id);
    }
  });
  return new Map([...rows].filter(([id]) => !known.has(id)).map(([id, row]) => [
    id, { ...row, source_aliases: aliases.get(id) },
  ]));
}

export function lane(row) {
  const p = row.path;
  if (/(?:^|\/)(?:tests|benches|testdata|fixtures)(?:\/|$)|\/workspace_tests\//.test(p)) return 'test_fixture_benchmark';
  if (/(?:^|\/)(?:verification|proofs|models|kani|loom|verus|flux)(?:\/|\.)|(?:_kani|_verus|_flux)\.rs$/.test(p)) return 'formal_model_candidate';
  if (/\/src\//.test(p) && /(?:^|\/)(?:test[^/]*|[^/]*tests?|proptest[^/]*|property_tests|lifecycle_tests|impl_tests)(?:\/|\.)/.test(p)) return 'embedded_test_candidate';
  return /\/src\//.test(p) ? 'production_bearing_pending' : 'crate_support_pending';
}

function collectReceipts(baseline, packets) {
  const raw = new Map();
  for (const packet of packets) for (const row of packet.rows) {
    const id = key(row);
    assert(baseline.has(id), `receipt outside baseline: ${id}`);
    if (packet.supersedes_key) {
      assert.equal(packet.rows.length, 1, 'amendment must contain exactly one tuple');
      assert.equal(packet.supersedes_key, id, `amendment changed tuple: ${id}`);
      assert(raw.has(id), `missing amendment predecessor: ${id}`);
    }
    if (raw.has(id)) {
      assert.equal(packet.supersedes_key, id, `unapproved duplicate: ${id}`);
      assert.equal(raw.get(id).at(-1).file, packet.supersedes_file, `wrong predecessor: ${id}`);
    }
    const versions = raw.get(id) || [];
    versions.push({ file: packet.file, sha256: packet.sha256, row });
    raw.set(id, versions);
  }
  return raw;
}

function requirePendingOwners(assessment, owners, id) {
  for (const pending of assessment.pending_intents) {
    assert(pending.intent && pending.reason && pending.owner, `unowned intent: ${id}`);
    const owner = owners.find(row => row.id === pending.owner);
    assert(owner && ['open', 'in_progress', 'blocked'].includes(owner.status), `inactive owner: ${pending.owner}`);
  }
}

const pendingFlags = ['production_pending', 'formal_pending', 'qualification_pending', 'test_or_proof_pending', 'overall_pending', 'pending'];
const pendingOwnerFields = ['pending_owner', 'qualification_owner'];
function obligationsAt(row, prefix) {
  const result = pendingFlags.filter(flag => row[flag] === true).map(flag => ({ id: `${prefix}.${flag}`, value: true }));
  for (const field of pendingOwnerFields) {
    if (row[field]) result.push({ id: `${prefix}.${field}`, value: row[field] });
  }
  for (const name of ['remaining_intents', 'pending_subintents', 'per_intent']) {
    for (const [index, value] of (row[name] || []).entries()) {
      if (name !== 'per_intent' || value.pending || value.pending_intent || value.pending_owner || value.qualification_owner) result.push({ id: `${prefix}.${name}[${index}]`, value });
    }
  }
  return result;
}

export function pendingObligations(row, prefix) {
  if (!row) return [];
  const result = obligationsAt(row, prefix);
  for (const name of ['resolution_r23', 'resolution_r24']) {
    if (row[name]) result.push(...obligationsAt(row[name], `${prefix}.${name}`));
  }
  return result;
}

function requirePendingCoverage(assessment, versions, id) {
  const expected = [...pendingObligations(versions.at(-1)?.row, 'receipt'), ...pendingObligations(assessment.original_audit, 'audit')];
  const actual = assessment.pending_intents.flatMap(intent => intent.source_obligations || []);
  assert.deepEqual(actual.sort((a, b) => a.id.localeCompare(b.id)), expected.sort((a, b) => a.id.localeCompare(b.id)), `dropped pending subintent: ${id}`);
  assessment.pending_intents.forEach(requireSourceOwner);
}

function requireSourceOwner(intent) {
  for (const source of intent.source_obligations || []) {
    for (const owner of sourceOwners(source)) {
      if (owner === intent.owner) continue;
      assert(intent.owner_rebindings?.some(binding => binding.source_owner === owner && binding.current_owner === intent.owner
        && binding.reason && binding.evidence?.length), `missing explicit owner rebinding: ${source.id}`);
    }
  }
}

export function sourceOwners(source) {
  if (typeof source.value === 'string' && pendingOwnerFields.some(field => source.id.endsWith(`.${field}`))) return [source.value];
  return [...new Set([source.value?.owner, source.value?.pending_owner, source.value?.qualification_owner,
    source.value?.current_active_owner?.id].filter(owner => typeof owner === 'string' && owner.length))];
}

function collectAssessments(baseline, assessments, owners, raw) {
  const result = new Map();
  for (const assessment of assessments) {
    const id = key(assessment);
    assert(baseline.has(id), `assessment outside baseline: ${id}`);
    assert(!result.has(id), `duplicate assessment: ${id}`);
    assert(assessment.disposition && assessment.reason, `missing disposition: ${id}`);
    assert(Array.isArray(assessment.evidence) && assessment.evidence.length, `missing evidence: ${id}`);
    assert(Array.isArray(assessment.pending_intents), `missing pending list: ${id}`);
    assert.deepEqual(assessment.source_aliases, baseline.get(id).source_aliases, `alias loss: ${id}`);
    requirePendingOwners(assessment, owners, id);
    requirePendingCoverage(assessment, raw.get(id) || [], id);
    result.set(id, {
      ...baseline.get(id), assessment,
      receipt_versions: raw.get(id) || [],
    });
  }
  for (const id of raw.keys()) assert(result.has(id), `unassessed receipt: ${id}`);
  return result;
}

export function applyReceipts(baseline, packets, assessments, owners) {
  return collectAssessments(baseline, assessments, owners, collectReceipts(baseline, packets));
}

const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const rows = raw => raw.trim().split('\n').filter(Boolean).map(JSON.parse);
function checkedInput(entry) {
  const bytes = fs.readFileSync(entry.file);
  assert.equal(digest(bytes), entry.sha256, `changed input: ${entry.file}`);
  return entry.format === 'json' ? JSON.parse(bytes) : rows(bytes.toString('utf8'));
}

function baselineFromInputs(manifest) {
  const inputs = Object.fromEntries(manifest.canonical.map(entry => [entry.name, checkedInput(entry)]));
  const baseline = canonical(inputs.groups, inputs.functional, [
    ...inputs.delegated, ...inputs.tokens, ...inputs.bodies, ...inputs.manual,
  ]);
  assert.equal(baseline.size, manifest.expected_baseline_tuples);
  const historical = new Set(inputs.historical_production.map(key));
  const actualProduction = [...baseline.values()].filter(row => lane(row) === 'production_bearing_pending');
  assert.deepEqual(new Set(actualProduction.map(key)), historical);
  return { baseline, actualProduction };
}

function reportFor(manifest, baseline, assessed, unreviewed, actualProduction) {
  return {
    anchor: manifest.anchor,
    baseline_tuples: baseline.size,
    baseline_production_tuples: actualProduction.length,
    assessed_tuples: assessed.size,
    assessed_with_pending_intents: [...assessed.values()].filter(row => row.assessment.pending_intents.length).length,
    remaining_unreviewed_tuples: unreviewed.length,
    remaining_unreviewed_by_lane: Object.fromEntries([...new Set(unreviewed.map(lane))].sort().map(name => [
      name, unreviewed.filter(row => lane(row) === name).length,
    ])),
    source_aliases_preserved: true,
    all_reconciliation_complete: unreviewed.length === 0 && [...assessed.values()].every(row => !row.assessment.pending_intents.length),
    engine_acceptance_claimed: false,
    proof_or_suite_execution_claimed: false,
  };
}

function writeArtifacts(directory, baseline, assessed, unreviewed, report) {
  fs.mkdirSync(path.join(directory, 'generated'), { recursive: true });
  for (const [name, value] of [
    ['baseline.jsonl', [...baseline.values()]],
    ['assessments.jsonl', [...assessed.values()]],
    ['unreviewed.jsonl', unreviewed],
  ]) fs.writeFileSync(path.join(directory, 'generated', name), value.map(row => JSON.stringify(row) + '\n').join(''));
  fs.writeFileSync(path.join(directory, 'generated', 'report.json'), JSON.stringify(report, null, 2) + '\n');
}

export function regenerate(directory) {
  const manifest = JSON.parse(fs.readFileSync(path.join(directory, 'input-manifest.json')));
  const { baseline, actualProduction } = baselineFromInputs(manifest);
  const packets = manifest.packets.map(entry => ({ ...entry, rows: checkedInput(entry) }));
  const assessments = manifest.assessments.flatMap(checkedInput);
  const owners = checkedInput(manifest.owners);
  const assessed = applyReceipts(baseline, packets, assessments, owners);
  const unreviewed = [...baseline].filter(([id]) => !assessed.has(id)).map(([, row]) => row);
  const report = reportFor(manifest, baseline, assessed, unreviewed, actualProduction);
  writeArtifacts(directory, baseline, assessed, unreviewed, report);
  return report;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) console.log(JSON.stringify(regenerate(path.dirname(fileURLToPath(import.meta.url)))));

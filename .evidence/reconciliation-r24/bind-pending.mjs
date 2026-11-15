import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { key, pendingObligations, sourceOwners } from './reconcile.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const manifestPath = path.join(directory, 'input-manifest.json');
const manifest = JSON.parse(fs.readFileSync(manifestPath));
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const readRows = entry => {
  const bytes = fs.readFileSync(entry.file);
  assert.equal(digest(bytes), entry.sha256, `changed input: ${entry.file}`);
  return bytes.toString('utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
};
const latest = new Map(manifest.packets.flatMap(entry => readRows(entry).map(row => [key(row), row])));

function matchingIntent(assessment, obligation) {
  return assessment.pending_intents.find(intent => obligation.value.intent && intent.intent === obligation.value.intent)
    || assessment.pending_intents.find(intent => obligation.value.reason && intent.reason.startsWith(obligation.value.reason))
    || (obligation.id.includes('formal') && assessment.pending_intents.find(intent => intent.owner === 'vb-god2f'))
    || assessment.pending_intents[0];
}

function bindObligation(assessment, obligation) {
  const intent = matchingIntent(assessment, obligation);
  assert(intent, `unassessed pending obligation: ${key(assessment)}`);
  intent.source_obligations.push(obligation);
  for (const owner of sourceOwners(obligation)) {
    if (owner !== intent.owner) intent.owner_rebindings.push({
      source_owner: owner, current_owner: intent.owner,
      reason: 'Retain historical owner verbatim and assign qualification to the current live scoped owner recorded in this assessment.',
      evidence: [manifest.owners.file, ...assessment.evidence.slice(0, 1)],
    });
  }
}

function bindAssessment(assessment) {
  assessment.pending_intents.forEach(intent => {
    intent.source_obligations = [];
    intent.owner_rebindings = [];
  });
  const obligations = [
    ...pendingObligations(latest.get(key(assessment)), 'receipt'),
    ...pendingObligations(assessment.original_audit, 'audit'),
  ];
  obligations.forEach(obligation => bindObligation(assessment, obligation));
  return assessment;
}

for (const entry of manifest.assessments) {
  const rows = readRows(entry).map(bindAssessment);
  const bytes = rows.map(row => JSON.stringify(row) + '\n').join('');
  fs.writeFileSync(entry.file, bytes);
  entry.sha256 = digest(bytes);
}
fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
console.log('Bound every declared pending source obligation and preserved explicit owner rebindings.');

import assert from 'node:assert/strict';
import test from 'node:test';
import { applyReceipts, canonical, key, lane, pendingObligations } from './reconcile.mjs';

const row = { path: 'crates/domain/src/lib.rs', old: '0'.repeat(40), new: 'a'.repeat(40), status: 'A' };
const groups = [
  { id: 'first', tips: ['tip1', 'tip2'], refs: ['branch1', 'workspace1'], changes: [row] },
  { id: 'second', tips: ['tip3'], refs: ['branch2'], changes: [row] },
];
const baseline = () => canonical(groups, {}, []);
const owners = [{ id: 'open-bead', status: 'in_progress' }, { id: 'closed-bead', status: 'closed' }];
const assessment = () => ({
  ...baseline().get(key(row)), disposition: 'mixed', reason: 'specific source comparison',
  evidence: ['current.rs'], pending_intents: [{ intent: 'native qualification', reason: 'valid fixture needed', owner: 'open-bead' }],
});
const packet = () => ({ file: 'original.jsonl', sha256: 'original-hash', rows: [{ ...row, original: 'retained' }] });

test('complete tuple identity distinguishes status and path as well as blobs', () => {
  assert.notEqual(key(row), key({ ...row, status: 'M' }));
  assert.notEqual(key(row), key({ ...row, path: 'crates/other/src/lib.rs' }));
  assert.notEqual(key(row), key({ ...row, old: 'b'.repeat(40) }));
  assert.notEqual(key(row), key({ ...row, new: 'b'.repeat(40) }));
  assert.throws(() => key({ ...row, old: 'truncated' }));
});

test('canonical deduplication retains every signature and every source alias', () => {
  assert.equal(baseline().size, 1);
  assert.deepEqual(baseline().get(key(row)).source_aliases, groups.map((group, index) => ({
    index, id: group.id, tips: group.tips, refs: group.refs,
  })));
});

test('one resolved signature resolves its shared exact tuple globally', () => {
  assert.equal(canonical(groups, { 1: ['specific outcome'] }, []).size, 0);
  assert.equal(canonical(groups, {}, [row]).size, 0);
  assert.equal(canonical(groups, {}, [{ ...row, status: 'M' }]).size, 1);
});

test('receipt import preserves original payload and owned mixed intent', () => {
  const result = applyReceipts(baseline(), [packet()], [assessment()], owners).get(key(row));
  assert.equal(result.receipt_versions[0].row.original, 'retained');
  assert.deepEqual(result.assessment.pending_intents, assessment().pending_intents);
  assert.deepEqual(result.source_aliases, assessment().source_aliases);
});

test('foreign receipt cannot silently match a path with different status', () => {
  assert.throws(() => applyReceipts(baseline(), [{ ...packet(), rows: [{ ...row, status: 'M' }] }], [assessment()], owners), /outside baseline/);
});

test('receipt cannot remove canonical source aliases', () => {
  assert.throws(() => applyReceipts(baseline(), [packet()], [{ ...assessment(), source_aliases: assessment().source_aliases.slice(1) }], owners), /alias loss/);
});

test('pending intent requires an existing active owner', () => {
  for (const owner of [undefined, 'tracker owner must be assigned', 'closed-bead']) {
    const changed = { ...assessment(), pending_intents: [{ intent: 'proof', reason: 'unqualified', owner }] };
    assert.throws(() => applyReceipts(baseline(), [packet()], [changed], owners), /unowned intent|inactive owner/);
  }
  assert.throws(() => applyReceipts(baseline(), [packet()], [assessment()], [{ id: 'open-bead' }]), /inactive owner/);
});

test('duplicate receipts require an exact append-only predecessor', () => {
  assert.throws(() => applyReceipts(baseline(), [packet(), packet()], [assessment()], owners), /unapproved duplicate/);
  const amendment = { file: 'followup.jsonl', sha256: 'followup-hash', rows: [{ ...row, amended: true }], supersedes_key: key(row), supersedes_file: 'original.jsonl' };
  const result = applyReceipts(baseline(), [packet(), amendment], [assessment()], owners).get(key(row));
  assert.equal(result.receipt_versions.length, 2);
  assert.equal(result.receipt_versions[0].row.original, 'retained');
  assert.equal(result.receipt_versions[1].row.amended, true);
  assert.throws(() => applyReceipts(baseline(), [packet(), { ...amendment, supersedes_file: 'wrong.jsonl' }], [assessment()], owners), /wrong predecessor/);
  const other = { ...row, path: 'crates/other/src/lib.rs' };
  const expanded = new Map([...baseline(), [key(other), { ...other, source_aliases: [] }]]);
  assert.throws(() => applyReceipts(expanded, [packet(), { ...amendment, rows: [...amendment.rows, other] }], [assessment()], owners), /exactly one tuple/);
  assert.throws(() => applyReceipts(expanded, [packet(), { ...amendment, rows: [other] }], [assessment()], owners), /changed tuple/);
  assert.throws(() => applyReceipts(baseline(), [amendment], [assessment()], owners), /missing amendment predecessor/);
});

test('every imported receipt needs one concrete assessment', () => {
  assert.throws(() => applyReceipts(baseline(), [packet()], [], owners), /unassessed receipt/);
  assert.throws(() => applyReceipts(baseline(), [packet()], [assessment(), assessment()], owners), /duplicate assessment/);
});

test('mixed raw receipt and original assessment cannot lose all pending intent', () => {
  const mixed = { ...packet(), rows: [{ ...row, resolution_r23: { formal_pending: true } }] };
  const dropped = { ...assessment(), pending_intents: [] };
  assert.throws(() => applyReceipts(baseline(), [mixed], [dropped], owners), /dropped pending subintent/);
  const source = { ...dropped, original_audit: { ...row, per_intent: [{ pending: true }] } };
  assert.throws(() => applyReceipts(baseline(), [], [source], owners), /dropped pending subintent/);
});

test('every declared pending subintent survives even with a false nested wrapper', () => {
  const raw = { ...row, pending: true, resolution_r23: { formal_pending: false }, remaining_intents: [
    { intent: 'A', owner: 'open-bead' }, { intent: 'B', owner: 'other-bead' },
  ] };
  const obligations = pendingObligations(raw, 'receipt');
  const firstOnly = { ...assessment(), pending_intents: [{ ...assessment().pending_intents[0], source_obligations: obligations.slice(0, 2) }] };
  assert.throws(() => applyReceipts(baseline(), [{ ...packet(), rows: [raw] }], [firstOnly], owners), /dropped pending subintent/);
  const misrouted = { ...assessment(), pending_intents: [{ ...assessment().pending_intents[0], source_obligations: obligations }] };
  assert.throws(() => applyReceipts(baseline(), [{ ...packet(), rows: [raw] }], [misrouted], owners), /owner rebinding/);
});

test('qualification owners declare obligations without boolean flags', () => {
  const raw = { ...row, pending_owner: 'historical-owner', per_intent: [{ intent: 'typed wrapper', qualification_owner: 'open-bead' }] };
  const obligations = pendingObligations(raw, 'receipt');
  assert.equal(obligations.length, 2);
  assert.throws(() => applyReceipts(baseline(), [{ ...packet(), rows: [raw] }], [{ ...assessment(), pending_intents: [] }], owners), /dropped pending subintent/);
  const changed = { ...assessment(), pending_intents: [{ ...assessment().pending_intents[0], source_obligations: obligations }] };
  assert.throws(() => applyReceipts(baseline(), [{ ...packet(), rows: [raw] }], [changed], owners), /owner rebinding/);
});

test('source-role lanes route without granting semantic closure', () => {
  const classify = p => lane({ ...row, path: p });
  assert.equal(classify('crates/domain/src/lib.rs'), 'production_bearing_pending');
  assert.equal(classify('crates/domain/src/tests.rs'), 'embedded_test_candidate');
  assert.equal(classify('crates/domain/src/verification/kani.rs'), 'formal_model_candidate');
  assert.equal(classify('crates/domain/tests/check.rs'), 'test_fixture_benchmark');
  assert.equal(classify('crates/domain/Cargo.toml'), 'crate_support_pending');
});

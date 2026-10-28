'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { compareOwnedContainerInventories: compare } = require('./helpers.cjs');
const id = n => n.toString(16).padStart(64, '0');
const row = (n, state) => ({ id: id(n), state });
const input = extra => ({ before: [], after: [], ownedIds: [], beforeComplete: true, afterComplete: true, ...extra });

test('distinguishes preexisting, stopped and absent owned IDs without mutation', () => {
  const data = input({ before: [row(1, 'running')], after: [row(1, 'running'), row(2, 'exited')], ownedIds: [id(2), id(3)] });
  const before = JSON.stringify(data);
  const r = compare(data);
  assert.equal(r.preexistingUnchanged, true);
  assert.deepEqual(r.ownedStopped, [id(2)]);
  assert.deepEqual(r.ownedAbsent, [id(3)]);
  assert.equal(r.allOwnedTerminalOrAbsent, true);
  assert.equal(r.status, 'compared');
  assert.equal(Object.hasOwn(r, 'safe_to_delete'), false);
  assert.equal(JSON.stringify(data), before);
});
test('active and unsettled states prevent terminal conclusion', () => {
  const r = compare(input({ after: [row(1, 'running'), row(2, 'paused'), row(3, 'restarting'), row(4, 'created')], ownedIds: [1, 2, 3, 4].map(id) }));
  assert.equal(r.counts.ownedActive, 3);
  assert.deepEqual(r.ownedUnsettled, [id(4)]);
  assert.equal(r.allOwnedTerminalOrAbsent, false);
});
test('preexisting ownership claims are conflicts, never adopted', () => {
  const r = compare(input({ before: [row(1, 'exited')], after: [row(1, 'running')], ownedIds: [id(1)] }));
  assert.deepEqual(r.ownershipConflicts, [id(1)]);
  assert.equal(r.counts.ownedActive, 0);
  assert.equal(r.allOwnedTerminalOrAbsent, false);
  assert.equal(r.preexistingUnchanged, false);
  assert.deepEqual(r.changedPreexisting, [{ id: id(1), before: 'exited', after: 'running' }]);
});
test('missing preexisting IDs and new unowned IDs are explicit', () => {
  const r = compare(input({ before: [row(1, 'exited')], after: [row(2, 'running')] }));
  assert.deepEqual(r.missingPreexisting, [id(1)]);
  assert.deepEqual(r.newUnowned, [id(2)]);
  assert.equal(r.preexistingUnchanged, false);
});
test('incomplete or undeclared snapshots do not prove absence', () => {
  for (const complete of [false, undefined, 'true']) {
    const r = compare(input({ ownedIds: [id(1)], afterComplete: complete }));
    assert.deepEqual(r, { status: 'inconclusive', reason: 'INCOMPLETE_INVENTORY' });
  }
});
test('supports Docker JSON field names explicitly', () => {
  const r = compare(input({ after: [{ ID: id(1), State: 'dead' }], ownedIds: [id(1)] }), { idKey: 'ID', stateKey: 'State' });
  assert.deepEqual(r.ownedStopped, [id(1)]);
});
test('truncation keeps full counts and negative conclusions', () => {
  const r = compare(input({ after: [row(3, 'running'), row(1, 'running'), row(2, 'running')], ownedIds: [id(1), id(2), id(3)] }), { maxOutputItems: 1 });
  assert.equal(r.truncated, true);
  assert.equal(r.counts.ownedActive, 3);
  assert.deepEqual(r.ownedActive, [id(1)]);
  assert.equal(r.allOwnedTerminalOrAbsent, false);
});
test('rejects duplicate snapshots and owned IDs', () => {
  assert.throws(() => compare(input({ after: [row(1, 'exited'), row(1, 'exited')] })), { code: 'DUPLICATE_CONTAINER_ID' });
  assert.throws(() => compare(input({ ownedIds: [id(1), id(1)] })), { code: 'DUPLICATE_OWNED_ID' });
});
test('rejects names, truncated IDs, unsupported states and inherited fields', () => {
  for (const value of ['fixture-name', 'abc123', 'a'.repeat(63)]) assert.throws(() => compare(input({ ownedIds: [value] })), { code: 'FULL_CONTAINER_ID_REQUIRED' });
  assert.throws(() => compare(input({ after: [row(1, 'unknown')] })), { code: 'INVALID_CONTAINER_STATE' });
  assert.throws(() => compare(input({ after: [Object.create(row(1, 'running'))] })), { code: 'INVALID_CONTAINER_ROW' });
});
test('rejects malformed or excessive inputs with bounded error codes', () => {
  assert.throws(() => compare(null), { code: 'INVALID_ARGUMENT' });
  assert.throws(() => compare(input(), null), { code: 'INVALID_ARGUMENT' });
  assert.throws(() => compare(input({ before: 'not-array' })), { code: 'INVALID_OR_EXCESSIVE_RECORDS' });
  assert.throws(() => compare(input({ after: [row(1, 'exited'), row(2, 'exited')] }), { maxRecords: 1 }), { code: 'INVALID_OR_EXCESSIVE_RECORDS' });
  assert.throws(() => compare(input(), { maxOutputItems: 0 }), { code: 'INVALID_LIMIT' });
  assert.throws(() => compare(input(), { idKey: 'bad.key' }), { code: 'INVALID_FIELD_KEY' });
});

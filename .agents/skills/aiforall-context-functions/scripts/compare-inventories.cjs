'use strict';

const states = new Set(['created', 'running', 'paused', 'restarting', 'removing', 'exited', 'dead']);
const activeStates = new Set(['running', 'paused', 'restarting']);
const terminalStates = new Set(['exited', 'dead']);
function fail(code) { const error = new Error(code); error.code = code; throw error; }
function integer(value, fallback, max) {
  const n = value === undefined ? fallback : value;
  if (!Number.isSafeInteger(n) || n < 1 || n > max) fail('INVALID_LIMIT');
  return n;
}
function id(value) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) fail('FULL_CONTAINER_ID_REQUIRED');
  return value;
}

/** Compare supplied, complete container snapshots. Never contact Docker or authorize deletion. */
function compareOwnedContainerInventories(input, options = {}) {
  if (!input || typeof input !== 'object' || Array.isArray(input)
    || !options || typeof options !== 'object' || Array.isArray(options)) fail('INVALID_ARGUMENT');
  const maxRecords = integer(options.maxRecords, 10000, 100000);
  const maxOutputItems = integer(options.maxOutputItems, 20, 1000);
  const idKey = options.idKey === undefined ? 'id' : options.idKey;
  const stateKey = options.stateKey === undefined ? 'state' : options.stateKey;
  if (![idKey, stateKey].every(k => typeof k === 'string' && /^[A-Za-z_][A-Za-z0-9_]{0,63}$/.test(k))) fail('INVALID_FIELD_KEY');
  for (const rows of [input.before, input.after, input.ownedIds]) {
    if (!Array.isArray(rows) || rows.length > maxRecords) fail('INVALID_OR_EXCESSIVE_RECORDS');
  }
  const normalize = rows => {
    const map = new Map();
    for (const row of rows) {
      if (!row || typeof row !== 'object' || Array.isArray(row)
        || !Object.hasOwn(row, idKey) || !Object.hasOwn(row, stateKey)) fail('INVALID_CONTAINER_ROW');
      const key = id(row[idKey]);
      if (!states.has(row[stateKey])) fail('INVALID_CONTAINER_STATE');
      if (map.has(key)) fail('DUPLICATE_CONTAINER_ID');
      map.set(key, row[stateKey]);
    }
    return map;
  };
  const before = normalize(input.before);
  const after = normalize(input.after);
  const owned = new Set();
  for (const item of input.ownedIds) {
    const key = id(item);
    if (owned.has(key)) fail('DUPLICATE_OWNED_ID');
    owned.add(key);
  }
  // Missing data is never treated as an empty, successful inventory.
  if (input.beforeComplete !== true || input.afterComplete !== true) {
    return { status: 'inconclusive', reason: 'INCOMPLETE_INVENTORY' };
  }
  const lists = { ownershipConflicts: [], ownedActive: [], ownedStopped: [], ownedUnsettled: [],
    ownedAbsent: [], missingPreexisting: [], changedPreexisting: [], newUnowned: [] };
  for (const key of owned) {
    if (before.has(key)) { lists.ownershipConflicts.push(key); continue; }
    if (!after.has(key)) { lists.ownedAbsent.push(key); continue; }
    const state = after.get(key);
    if (activeStates.has(state)) lists.ownedActive.push(key);
    else if (terminalStates.has(state)) lists.ownedStopped.push(key);
    else lists.ownedUnsettled.push(key);
  }
  for (const [key, state] of before) {
    if (!after.has(key)) lists.missingPreexisting.push(key);
    else if (after.get(key) !== state) lists.changedPreexisting.push({ id: key, before: state, after: after.get(key) });
  }
  for (const key of after.keys()) if (!before.has(key) && !owned.has(key)) lists.newUnowned.push(key);
  const counts = {};
  const bounded = {};
  let truncated = false;
  for (const [key, values] of Object.entries(lists)) {
    values.sort((a, b) => (typeof a === 'string' ? a : a.id).localeCompare(typeof b === 'string' ? b : b.id));
    counts[key] = values.length;
    bounded[key] = values.slice(0, maxOutputItems);
    truncated ||= values.length > maxOutputItems;
  }
  return { status: 'compared', beforeCount: before.size, afterCount: after.size, ownedDeclaredCount: owned.size,
    preexistingUnchanged: counts.missingPreexisting === 0 && counts.changedPreexisting === 0,
    allOwnedTerminalOrAbsent: counts.ownershipConflicts === 0 && counts.ownedActive === 0 && counts.ownedUnsettled === 0,
    counts, ...bounded, truncated };
}

module.exports = { compareOwnedContainerInventories };

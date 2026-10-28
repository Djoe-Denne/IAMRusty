'use strict';

const helpers = require('./helpers.cjs');
const catalogue = require('../catalogue.json');

// Explicit allowlist; never load code from catalogue paths or candidates/.
const operations = Object.freeze({
  'read-evidence': input => helpers.readEvidence(input),
  'classify-command': input => helpers.classifyCommandResult(input.result, input.options),
  'compare-owned-container-inventories': input => helpers.compareOwnedContainerInventories(input.inventory, input.options),
});

async function readInput() {
  let bytes = 0;
  const parts = [];
  for await (const part of process.stdin) {
    bytes += part.length;
    if (bytes > 65536) throw new Error('INPUT_TOO_LARGE');
    parts.push(part);
  }
  return JSON.parse(Buffer.concat(parts).toString('utf8'));
}

async function main() {
  const [command, id, ...extra] = process.argv.slice(2);
  if (extra.length || !['list', 'describe', 'invoke'].includes(command)) throw new Error('INVALID_COMMAND');
  if (command === 'list') {
    if (id !== undefined) throw new Error('INVALID_COMMAND');
    return catalogue.functions.filter(entry => entry.status === 'validated')
      .map(({ id, purpose, cli }) => ({ id, purpose, cli }));
  }
  const entry = catalogue.functions.find(item => item.id === id && item.status === 'validated');
  if (!entry) throw new Error('UNKNOWN_FUNCTION');
  if (command === 'describe') return entry;
  if (!Object.hasOwn(operations, id)) throw new Error('MODULE_ONLY_FUNCTION');
  const input = await readInput();
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new Error('INVALID_ARGUMENT');
  return operations[id](input);
}

main().then(value => {
  console.log(JSON.stringify(value));
  if (value?.ok === false) process.exitCode = 1;
}).catch(error => {
  // Never return a stack or arbitrary exception message, which may include input data.
  const known = ['INPUT_TOO_LARGE', 'INVALID_COMMAND', 'UNKNOWN_FUNCTION', 'MODULE_ONLY_FUNCTION', 'INVALID_ARGUMENT'];
  const code = typeof error.code === 'string' && /^[A-Z0-9_]{1,64}$/.test(error.code)
    ? error.code : known.includes(error.message) ? error.message : 'INVALID_INPUT_OR_OPERATION_FAILED';
  console.log(JSON.stringify({ ok: false, error: { code } }));
  process.exitCode = 1;
});

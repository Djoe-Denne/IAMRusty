'use strict';

const test = require('node:test');
const { after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { readEvidence, classifyCommandResult, collectIndependentReads } = require('./helpers.cjs');

// Only fixtures created by this test suite are removed; never project files.
const approvedTemp = process.platform === 'win32'
  ? path.join(process.env.LOCALAPPDATA, 'Temp', 'opencode') : require('node:os').tmpdir();
const owned = fs.mkdtempSync(path.join(approvedTemp, 'context-functions-test-'));
const root = path.join(owned, 'project with spaces é');
fs.mkdirSync(root);
fs.writeFileSync(path.join(root, 'texte é.txt'), 'alpha\r\nbêta\r\ngamma\r\n');
fs.writeFileSync(path.join(root, 'lf.txt'), 'alpha\nbeta\ngamma');
fs.writeFileSync(path.join(root, 'cr.txt'), 'alpha\rbeta\r');
fs.writeFileSync(path.join(root, 'empty.txt'), '');
fs.writeFileSync(path.join(root, 'binary.bin'), Buffer.from([1, 0, 2]));
fs.writeFileSync(path.join(root, 'invalid.txt'), Buffer.from([0xc3, 0x28]));
fs.writeFileSync(path.join(root, 'long.txt'), 'x'.repeat(10000));
fs.mkdirSync(path.join(root, 'folder'));
const runner = path.join(__dirname, 'run.cjs');
const catalogue = require('../catalogue.json');
after(() => fs.rmSync(owned, { recursive: true, force: true }));

function read(files, options = {}) {
  return readEvidence({ root, files, ...options });
}

function cli(args, input, script = runner) {
  const result = spawnSync(process.execPath, [script, ...args], {
    input: input === undefined ? undefined : typeof input === 'string' ? input : JSON.stringify(input),
    encoding: 'utf8', timeout: 5000, maxBuffer: 100000, windowsHide: true,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.stderr, '');
  return { status: result.status, value: JSON.parse(result.stdout) };
}

test('comparative evidence preserves paths, line numbers, spaces and Unicode', () => {
  const result = read([
    { path: 'texte é.txt', ranges: [{ start: 2, end: 3 }] },
    { path: 'lf.txt', ranges: [{ start: 1, end: 1 }] },
  ]);
  assert.equal(result.ok, true);
  assert.equal(result.partial, false);
  assert.equal(result.entries[0].content, '2: bêta\n3: gamma');
  assert.equal(result.entries[0].lineCount, 3);
  assert.equal(result.entries[1].content, '1: alpha');
});

test('LF, CRLF, CR, empty files and absent final newline are handled', () => {
  const result = read(['lf.txt', 'texte é.txt', 'cr.txt', 'empty.txt'].map(path => ({ path })));
  assert.equal(result.ok, true);
  assert.deepEqual(result.entries.map(entry => entry.lineCount), [3, 3, 2, 0]);
  assert.equal(result.entries[3].content, '');
});

test('missing path and directory preserve neighboring successes', () => {
  const result = read([{ path: 'missing.txt' }, { path: 'folder' }, { path: 'lf.txt' }]);
  assert.equal(result.ok, false);
  assert.equal(result.partial, true);
  assert.deepEqual(result.entries.map(entry => entry.status), ['error', 'error', 'ok']);
  assert.equal(result.entries[0].error.code, 'ENOENT');
  assert.equal(result.entries[1].error.code, 'NOT_A_FILE');
});

test('relative or absent root is rejected without treating it as empty evidence', () => {
  assert.throws(() => readEvidence({ root: '.', files: [{ path: 'lf.txt' }] }), { code: 'INVALID_ROOT' });
  assert.throws(() => readEvidence({ root: path.join(owned, 'absent'), files: [{ path: 'lf.txt' }] }), { code: 'ENOENT' });
});

test('absolute paths, traversal, drive-relative Windows paths and NUL rejected', () => {
  const result = read([
    { path: '../outside.txt' }, { path: path.join(root, 'lf.txt') },
    { path: 'C:relative.txt' }, { path: 'C:\\outside.txt' }, { path: 'bad\0name' },
  ]);
  assert.equal(result.ok, false);
  assert(result.entries.every(entry => entry.status === 'error'));
  assert.deepEqual(result.entries.map(entry => entry.error.code), [
    'PATH_OUTSIDE_ROOT', 'PATH_OUTSIDE_ROOT', 'PATH_OUTSIDE_ROOT', 'PATH_OUTSIDE_ROOT', 'INVALID_PATH',
  ]);
});

test('existing symlink/junction outside root is rejected', t => {
  const external = path.join(owned, 'external');
  const link = path.join(root, 'external-link');
  fs.mkdirSync(external);
  fs.writeFileSync(path.join(external, 'private.txt'), 'DO_NOT_RETURN');
  try {
    fs.symlinkSync(external, link, process.platform === 'win32' ? 'junction' : 'dir');
  } catch (error) {
    if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) return t.skip('Host cannot create test link: ' + error.code);
    throw error;
  }
  const result = read([{ path: 'external-link/private.txt' }]);
  assert.equal(result.entries[0].error.code, 'PATH_OUTSIDE_ROOT');
  assert(!JSON.stringify(result).includes('DO_NOT_RETURN'));
});

test('invalid entries, ranges and budgets are explicit errors', () => {
  for (const input of [null, {}, [], { root, files: [] }, { root, files: [{ path: 'lf.txt' }], maxOutputChars: 0 }]) {
    assert.throws(() => readEvidence(input));
  }
  const result = read([null, { path: 'lf.txt', ranges: [] }, { path: 'lf.txt', ranges: [{ start: 0, end: 2 }] }]);
  assert(result.entries.every(entry => entry.status === 'error'));
});

test('out-of-bounds range is not silently empty, range end reports actual lines', () => {
  const result = read([
    { path: 'lf.txt', ranges: [{ start: 9, end: 10 }] },
    { path: 'lf.txt', ranges: [{ start: 2, end: 99 }] },
  ]);
  assert.equal(result.entries[0].error.code, 'RANGE_OUT_OF_BOUNDS');
  assert.deepEqual(result.entries[1].returnedRanges, [{ start: 2, end: 3 }]);
});

test('file size budget, binary files and invalid UTF-8 are not source evidence', () => {
  const result = read([{ path: 'long.txt' }, { path: 'binary.bin' }, { path: 'invalid.txt' }, { path: 'lf.txt' }], { maxFileBytes: 100 });
  assert.equal(result.entries[0].error.code, 'FILE_TOO_LARGE');
  assert.equal(result.entries[1].error.code, 'BINARY_FILE');
  assert.equal(result.entries[2].status, 'error');
  assert.equal(result.entries[3].status, 'ok');
});

test('output truncation is explicit and never splits a numbered line', () => {
  const result = read([{ path: 'lf.txt' }, { path: 'lf.txt' }], { maxOutputChars: 8 });
  assert.equal(result.ok, false);
  assert.equal(result.partial, true);
  assert.equal(result.truncated, true);
  assert.equal(result.outputChars, 8);
  assert.equal(result.entries[0].content, '1: alpha');
  assert.equal(result.entries[0].truncated, true);
  assert.equal(result.entries[1].error.code, 'OUTPUT_BUDGET');
});

test('a line larger than output budget returns no misleading partial content', () => {
  const result = read([{ path: 'long.txt' }, { path: 'lf.txt', ranges: [{ start: 1, end: 1 }] }], { maxOutputChars: 8 });
  assert.equal(result.entries[0].content, '');
  assert.equal(result.entries[0].truncated, true);
  assert.equal(result.entries[1].content, '1: alpha');
});

test('total read budget skips later file with an explicit reason', () => {
  const size = fs.statSync(path.join(root, 'lf.txt')).size;
  const result = read([{ path: 'lf.txt' }, { path: 'lf.txt' }], { maxTotalBytes: size });
  assert.equal(result.bytesRead, size);
  assert.equal(result.entries[1].status, 'skipped');
  assert.equal(result.entries[1].error.code, 'READ_BUDGET');
  assert.equal(result.truncated, true);
});

test('failed binary/UTF-8 processing still consumes the total read budget', () => {
  const result = read([{ path: 'binary.bin' }, { path: 'invalid.txt' }, { path: 'lf.txt' }], { maxTotalBytes: 5 });
  assert.equal(result.bytesRead, 5);
  assert.equal(result.entries[0].error.code, 'BINARY_FILE');
  assert.equal(result.entries[1].status, 'error');
  assert.equal(result.entries[2].error.code, 'READ_BUDGET');
});

test('rg exit codes 0/1/2 are distinct; another program exit 1 is not no-match', () => {
  const states = [0, 1, 2].map(status => classifyCommandResult({ status }, { noMatchExitCodes: [1] }));
  assert.deepEqual(states.map(result => result.kind), ['success', 'no-match', 'command-error']);
  assert.deepEqual(states.map(result => result.ok), [true, true, false]);
  assert.equal(classifyCommandResult({ status: 1 }).kind, 'command-error');
});

test('missing executable, timeout, maxBuffer, signal and unknown are distinct', () => {
  const cases = [
    [{ status: null, error: { code: 'ENOENT' } }, 'executable-not-found'],
    [{ status: null, error: { code: 'ETIMEDOUT' }, signal: 'SIGTERM' }, 'timeout'],
    [{ status: null, error: { code: 'ENOBUFS' } }, 'output-limit'],
    [{ status: null, error: { code: 'ERR_CHILD_PROCESS_STDIO_MAXBUFFER' } }, 'output-limit'],
    [{ status: null, error: { code: 'EACCES' } }, 'execution-error'],
    [{ status: null, signal: 'SIGTERM' }, 'signal'],
    [{ status: null }, 'unknown'],
  ];
  for (const [input, expected] of cases) assert.equal(classifyCommandResult(input).kind, expected);
});

test('raw stdout, stderr, exception stack and message are never returned by default', () => {
  const secret = 'PRIVATE_RAW_OUTPUT';
  const error = Object.assign(new Error(secret), { code: 'ENOENT', stdout: secret, stderr: secret });
  const result = classifyCommandResult({ status: null, error, stdout: secret, stderr: secret });
  assert(!JSON.stringify(result).includes(secret));
  assert.deepEqual(result, { ok: false, kind: 'executable-not-found', exitCode: null, signal: null, errorCode: 'ENOENT' });
});

test('explicit diagnostic is bounded and truncation flagged', () => {
  const result = classifyCommandResult({ status: 2 }, { diagnosticText: 'chosen diagnostic', maxDiagnosticChars: 6 });
  assert.equal(result.diagnostic, 'chosen');
  assert.equal(result.diagnosticTruncated, true);
  assert.throws(() => classifyCommandResult({ status: '0' }));
  assert.throws(() => classifyCommandResult({ status: 0 }, { noMatchExitCodes: [0] }));
  assert.throws(() => classifyCommandResult({ status: 0 }, { diagnosticText: {} }));
});

test('independent reads preserve sync throw, rejection, fulfilled business error and order', async () => {
  const seen = [];
  const results = await collectIndependentReads([
    { label: 'success', run: async () => { seen.push('success'); return 7; } },
    { label: 'sync-error', run: () => { throw Object.assign(new Error('PRIVATE'), { code: 'ENOENT' }); } },
    { label: 'rejection', run: async () => { throw new Error('PRIVATE'); } },
    { label: 'business-error', run: async () => ({ isError: true }) },
    { label: 'late', run: async () => { await Promise.resolve(); seen.push('late'); return 'finished'; } },
  ]);
  assert.deepEqual(results.map(result => result.label), ['success', 'sync-error', 'rejection', 'business-error', 'late']);
  assert.deepEqual(results.map(result => result.status), ['fulfilled', 'rejected', 'rejected', 'fulfilled', 'fulfilled']);
  assert.deepEqual(results[1].error, { code: 'ENOENT' });
  assert.deepEqual(results[3].value, { isError: true });
  assert.deepEqual(seen, ['success', 'late']);
  assert(!JSON.stringify(results).includes('PRIVATE'));
});

test('independent read labels and callbacks are validated before any operation runs', async () => {
  let called = false;
  await assert.rejects(collectIndependentReads([
    { label: 'same', run: () => { called = true; } }, { label: 'same', run: () => {} },
  ]), { code: 'INVALID_ARGUMENT' });
  assert.equal(called, false);
  await assert.rejects(collectIndependentReads([{ label: 'missing' }]));
});

test('catalogue documents exactly the exported functions and their evidence', () => {
  assert.equal(catalogue.schemaVersion, 1);
  assert.equal(new Set(catalogue.functions.map(entry => entry.id)).size, catalogue.functions.length);
  assert.deepEqual(catalogue.functions.map(entry => entry.export).sort(), Object.keys(require('./helpers.cjs')).sort());
  for (const entry of catalogue.functions) {
    assert.equal(entry.status, 'validated');
    assert.equal(entry.evidence, 'VALIDATION.md');
    assert(entry.purpose && entry.parameters && entry.limits);
  }
});

test('CLI discovers short metadata and describes a function without loading its source', () => {
  const list = cli(['list']);
  assert.equal(list.status, 0);
  assert.deepEqual(list.value.map(entry => entry.id), catalogue.functions.map(entry => entry.id));
  assert(list.value.every(entry => !Object.hasOwn(entry, 'export')));
  assert.equal(cli(['describe', 'read-evidence']).value.export, 'readEvidence');
});

test('CLI accepts JSON stdin, spaces/Unicode and invocation of read evidence', () => {
  const result = cli(['invoke', 'read-evidence'], { root, files: [{ path: 'texte é.txt', ranges: [{ start: 2, end: 2 }] }] });
  assert.equal(result.status, 0);
  assert.equal(result.value.entries[0].content, '2: bêta');
  const noMatch = cli(['invoke', 'classify-command'], { result: { status: 1 }, options: { noMatchExitCodes: [1] } });
  assert.equal(noMatch.status, 0);
  assert.equal(noMatch.value.kind, 'no-match');
});

test('CLI failures are structured, bounded and do not echo malformed input', () => {
  const malformed = cli(['invoke', 'read-evidence'], '{PRIVATE_INVALID_JSON');
  assert.equal(malformed.status, 1);
  assert(!JSON.stringify(malformed.value).includes('PRIVATE'));
  assert.equal(cli(['invoke', 'classify-command'], { result: { status: 2 } }).status, 1);
  assert.equal(cli(['invoke', 'read-evidence'], 'x'.repeat(65537)).value.error.code, 'INPUT_TOO_LARGE');
  assert.equal(cli(['invoke', 'collect-independent-reads'], {}).value.error.code, 'MODULE_ONLY_FUNCTION');
  assert.equal(cli(['list', 'unexpected']).value.error.code, 'INVALID_COMMAND');
});

test('promoted inventory CLI matches module and describes both new exports', () => {
  const inventory = { before: [], after: [], ownedIds: [], beforeComplete: true, afterComplete: true };
  const helpers = require('./helpers.cjs');
  assert.deepEqual(cli(['invoke', 'compare-owned-container-inventories'], { inventory }).value,
    helpers.compareOwnedContainerInventories(inventory));
  assert.equal(cli(['describe', 'decode-evidence-text']).value.export, 'decodeEvidenceText');
  assert.equal(cli(['describe', 'compare-owned-container-inventories']).value.cli, true);
  assert.equal(cli(['invoke', 'decode-evidence-text'], {}).value.error.code, 'MODULE_ONLY_FUNCTION');
});

test('inventory CLI preserves inconclusive results and sanitizes invalid IDs', () => {
  const unknown = cli(['invoke', 'compare-owned-container-inventories'], { inventory: { before: [], after: [], ownedIds: [] } });
  assert.equal(unknown.status, 0); // Transport succeeded, business result remains inconclusive.
  assert.equal(unknown.value.status, 'inconclusive');
  const invalid = cli(['invoke', 'compare-owned-container-inventories'], { inventory: { before: [], after: [], ownedIds: ['PRIVATE_ID'] } });
  assert.equal(invalid.status, 1);
  assert.equal(invalid.value.error.code, 'FULL_CONTAINER_ID_REQUIRED');
  assert(!JSON.stringify(invalid.value).includes('PRIVATE_ID'));
  assert.equal(cli(['invoke', 'compare-owned-container-inventories'], {}).value.error.code, 'INVALID_ARGUMENT');
});

test('candidate code is never discovered/imported and registry cannot dispatch arbitrary modules', () => {
  const skill = path.join(owned, 'quarantine-skill');
  fs.mkdirSync(path.join(skill, 'scripts'), { recursive: true });
  fs.mkdirSync(path.join(skill, 'candidates', 'poison'), { recursive: true });
  fs.copyFileSync(runner, path.join(skill, 'scripts', 'run.cjs'));
  fs.copyFileSync(path.join(__dirname, 'helpers.cjs'), path.join(skill, 'scripts', 'helpers.cjs'));
  for (const file of ['compare-inventories.cjs', 'decode-evidence.cjs']) {
    fs.copyFileSync(path.join(__dirname, file), path.join(skill, 'scripts', file));
  }
  const injected = structuredClone(catalogue);
  injected.functions.push({ id: 'poison', status: 'candidate', cli: true, module: 'candidates/poison/function.cjs' });
  fs.writeFileSync(path.join(skill, 'catalogue.json'), JSON.stringify(injected));
  fs.writeFileSync(path.join(skill, 'candidates', 'poison', 'function.cjs'), "throw new Error('POISON_EXECUTED');");
  const isolatedRunner = path.join(skill, 'scripts', 'run.cjs');
  assert(!cli(['list'], undefined, isolatedRunner).value.some(entry => entry.id === 'poison'));
  assert.equal(cli(['invoke', 'poison'], {}, isolatedRunner).value.error.code, 'UNKNOWN_FUNCTION');
  injected.functions.at(-1).status = 'validated';
  fs.writeFileSync(path.join(skill, 'catalogue.json'), JSON.stringify(injected));
  assert.equal(cli(['invoke', 'poison'], {}, isolatedRunner).value.error.code, 'MODULE_ONLY_FUNCTION');
  assert.equal(cli(['invoke', '../candidates/poison/function.cjs'], {}, isolatedRunner).value.error.code, 'UNKNOWN_FUNCTION');
});

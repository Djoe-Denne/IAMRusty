'use strict';

const test = require('node:test');
const { after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { inspectTrace, inspectTraceFile } = require('./inspect-trace.cjs');

const temp = process.platform === 'win32' ? path.join(process.env.LOCALAPPDATA, 'Temp', 'opencode') : require('node:os').tmpdir();
const owned = fs.mkdtempSync(path.join(temp, 'context-ingest-test-'));
after(() => fs.rmSync(owned, { recursive: true, force: true }));
const script = path.join(__dirname, 'inspect-trace.cjs');
const sha = value => crypto.createHash('sha256').update(value).digest('hex');

function tool(name, input, state = {}) {
  return { type: 'tool', id: 'call_safe', name, state: { status: 'completed', input, ...state } };
}

function message(content, id = 'msg_safe') {
  return { id, type: 'assistant', content };
}

function file(name, bytes) {
  const target = path.join(owned, name);
  fs.writeFileSync(target, bytes);
  return target;
}

function cli(args) {
  const r = spawnSync(process.execPath, [script, ...args], { encoding: 'utf8', timeout: 5000, maxBuffer: 1000000, windowsHide: true });
  assert.equal(r.error, undefined);
  assert.equal(r.stderr, '');
  return { exit: r.status, value: JSON.parse(r.stdout) };
}

test('native assistant tool inputs produce metadata only, not source or results', () => {
  const code = 'console.log("PRIVATE_SOURCE")';
  const result = inspectTrace([message([tool('ctx_execute', { language: 'javascript', code }, {
    exitCode: 0, content: [{ type: 'text', text: 'PRIVATE_RESULT' }], error: { message: 'PRIVATE_ERROR' },
  })])]);
  assert.equal(result.totalScriptRecords, 1);
  assert.equal(result.records[0].scriptSha256, sha(code));
  assert.equal(result.records[0].outcome, 'success-reported');
  assert.equal(result.records[0].requiresManualReview, true);
  assert(!JSON.stringify(result).includes('PRIVATE_'));
});

test('client orchestration remains unattributed even with completed and exit0', () => {
  const result = inspectTrace([message([tool('execute', { code: 'const f = tools.any; await f({});' }, { exitCode: 0 })])]);
  assert.equal(result.records[0].kind, 'client-orchestration');
  assert.equal(result.records[0].outcome, 'unattributed');
});

test('structured rejected/isError markers are review hints, not automatic success or total failure', () => {
  const result = inspectTrace([message([tool('execute', { code: 'await Promise.allSettled(jobs);' }, {
    content: [{ type: 'text', text: JSON.stringify([{ status: 'fulfilled', value: 1 }, { status: 'rejected', reason: 'PRIVATE' }, { isError: true }]) }],
  })])]);
  assert.deepEqual(result.records[0].structuredErrorMarkers, { rejected: 1, isError: 1, scanLimited: false });
  assert.equal(result.records[0].outcome, 'unattributed');
  assert(!JSON.stringify(result).includes('PRIVATE'));
});

test('a string containing error examples is not parsed as proof of execution failure', () => {
  const result = inspectTrace([message([tool('ctx_execute_file', { language: 'python', code: 'print(FILE_CONTENT)' }, {
    content: [{ type: 'text', text: 'source example: Exit code: 1 and isError:true' }],
  })])]);
  assert.equal(result.records[0].outcome, 'completed-unverified');
  assert.equal(result.records[0].structuredErrorMarkers.rejected, 0);
  assert.equal(result.records[0].structuredErrorMarkers.isError, 0);
});

test('tool error/cancellation, explicit failure and pending are distinct', () => {
  const result = inspectTrace([message([
    tool('shell', { command: 'echo test' }, { status: 'error', error: { message: 'PRIVATE' } }),
    tool('ctx_execute', { code: 'x', language: 'javascript' }, { status: 'cancelled' }),
    tool('shell', { command: 'exit 2' }, { exitCode: 2 }),
    tool('shell', { command: 'echo pending' }, { status: 'running' }),
  ])]);
  assert.deepEqual(result.records.map(r => r.outcome), ['tool-error-reported', 'tool-error-reported', 'failure-reported', 'pending-or-unknown']);
  assert(!JSON.stringify(result).includes('PRIVATE'));
});

test('only assistant executions count; user code, prompts and reasoning are ignored', () => {
  const fake = tool('shell', { command: 'PRIVATE_USER_COMMAND' });
  const result = inspectTrace([
    { type: 'user', text: 'PRIVATE_PROMPT', content: [fake] },
    message([{ type: 'reasoning', text: 'PRIVATE_REASONING' }, { type: 'text', text: 'PRIVATE_ANSWER' }, tool('read', { path: 'PRIVATE_PATH' })]),
    { type: 'idle' },
  ]);
  assert.equal(result.totalScriptRecords, 0);
  assert.equal(result.toolCalls, 1);
  assert(!JSON.stringify(result).includes('PRIVATE'));
});

test('extracting a script never executes it or imports candidates', () => {
  const sentinel = path.join(owned, 'must-not-exist.txt');
  const code = 'require("node:fs").writeFileSync(' + JSON.stringify(sentinel) + ', "PRIVATE");';
  const result = inspectTrace([message([tool('execute', { code })])]);
  assert.equal(result.totalScriptRecords, 1);
  assert.equal(fs.existsSync(sentinel), false);
  assert(!JSON.stringify(result).includes(sentinel));
});

test('direct context tool namespaces and language allowlist are recognized conservatively', () => {
  const result = inspectTrace([message([
    tool('context-mode.ctx_execute', { language: 'typescript', code: 'console.log(1)' }),
    tool('mcp_ctx_execute_file', { language: 'PRIVATE_LANGUAGE', code: 'x' }),
  ])]);
  assert.deepEqual(result.records.map(r => r.kind), ['context-script', 'context-script']);
  assert.deepEqual(result.records.map(r => r.language), ['typescript', 'unknown']);
  assert(!JSON.stringify(result).includes('PRIVATE_LANGUAGE'));
});

test('duplicate exact scripts are grouped without claiming semantic reusability', () => {
  const result = inspectTrace([message([
    tool('shell', { command: 'echo same' }), tool('shell', { command: 'echo same' }),
    tool('execute', { code: 'echo same' }),
  ])]);
  assert.equal(result.repeatedExactScripts.length, 1);
  assert.equal(result.repeatedExactScripts[0].count, 2);
  assert.deepEqual(result.repeatedExactScripts[0].records, ['0:0', '0:1']);
  assert.equal(result.records.every(r => r.requiresManualReview), true);
});

test('output pagination is explicit and retains total count', () => {
  const trace = [message(Array.from({ length: 5 }, (_, i) => tool('shell', { command: 'echo ' + i })))];
  const first = inspectTrace(trace, { offset: 0, limit: 2 });
  const last = inspectTrace(trace, { offset: 4, limit: 2 });
  assert.equal(first.totalScriptRecords, 5);
  assert.equal(first.returnedRecords, 2);
  assert.equal(first.truncatedRecords, true);
  assert.equal(last.records[0].record, '0:4');
  assert.equal(last.truncatedRecords, false);
});

test('completeness metadata does not invent proof of complete pagination', () => {
  const page = inspectTrace({ data: [message([])], cursor: { next: 'opaque' } });
  const snapshot = inspectTrace({ messages: [message([]), { type: 'idle' }], pages: 2 });
  assert.equal(page.completeness.nextCursorPresent, true);
  assert.equal(page.completeness.declaredComplete, false);
  assert.equal(snapshot.completeness.pagesReported, 2);
  assert.equal(snapshot.completeness.hasFinalIdle, true);
  assert.equal(snapshot.completeness.declaredComplete, false);
});

test('unsupported, ambiguous and malformed trace shapes are rejected or signaled', () => {
  for (const trace of [null, '', {}, { messages: [], data: [] }, [null], [message({})]]) assert.throws(() => inspectTrace(trace));
  const unknown = inspectTrace([{ role: 'assistant', content: [] }]);
  assert.equal(unknown.unsupportedMessageCount, 1);
  assert.equal(unknown.totalScriptRecords, 0);
});

test('empty and non-code tool inputs do not become functions', () => {
  const result = inspectTrace([message([
    tool('execute', { code: ' ' }), tool('grep', { pattern: 'anything' }),
    tool('shell', { command: null }), tool('read', { path: 'file' }),
  ])]);
  assert.equal(result.toolCalls, 4);
  assert.equal(result.totalScriptRecords, 0);
});

test('invalid options, message and tool limits fail before publishing an inventory', () => {
  for (const options of [null, [], { offset: -1 }, { limit: 0 }, { limit: 201 }]) assert.throws(() => inspectTrace([], options));
  assert.throws(() => inspectTrace(Array.from({ length: 5001 }, () => ({ type: 'idle' }))), { code: 'TOO_MANY_MESSAGES' });
  assert.throws(() => inspectTrace([message(Array.from({ length: 10001 }, () => tool('read', {})))]), { code: 'TOO_MANY_TOOL_CALLS' });
});

test('deep or oversized structured results declare incomplete marker scanning', () => {
  let value = { isError: true };
  for (let i = 0; i < 20; i++) value = { child: value };
  const result = inspectTrace([message([
    tool('execute', { code: 'x' }, { content: [{ type: 'text', text: JSON.stringify(value) }] }),
    tool('execute', { code: 'y' }, { content: [{ type: 'text', text: '[' + ' '.repeat(2097153) + ']' }] }),
  ])]);
  assert(result.records.every(r => r.structuredErrorMarkers.scanLimited));
});

test('unsafe metadata identifiers are replaced instead of echoing text', () => {
  const result = inspectTrace([message([tool('shell', { command: 'echo safe' })], 'PRIVATE MESSAGE WITH SPACES')]);
  assert.equal(result.records[0].messageID, 'unknown-message');
  assert(!JSON.stringify(result).includes('PRIVATE MESSAGE'));
});

test('file inspection supports spaces/Unicode and hashes the source without changing it', () => {
  const bytes = JSON.stringify({ messages: [message([tool('shell', { command: 'echo secret' })])], pages: 2 });
  const source = file('trace é espace.json', bytes);
  const result = inspectTraceFile(source);
  assert.equal(result.sourceSha256, sha(bytes));
  assert.equal(result.sourceBytes, Buffer.byteLength(bytes));
  assert.equal(fs.readFileSync(source, 'utf8'), bytes);
  assert(!JSON.stringify(result).includes('echo secret'));
});

test('file path/type/size, invalid JSON, binary and malformed UTF-8 are guarded', () => {
  assert.throws(() => inspectTraceFile('relative.json'), { code: 'ABSOLUTE_PATH_REQUIRED' });
  assert.throws(() => inspectTraceFile(path.join(owned, 'absent.json')), { code: 'ENOENT' });
  assert.throws(() => inspectTraceFile(owned));
  const valid = file('valid.json', '[]');
  assert.throws(() => inspectTraceFile(valid, { maxBytes: 1 }), { code: 'INPUT_TOO_LARGE' });
  assert.throws(() => inspectTraceFile(valid, null), { code: 'INVALID_ARGUMENT' });
  assert.throws(() => inspectTraceFile(file('invalid.json', '{PRIVATE')), { code: 'INVALID_JSON_OR_UTF8' });
  assert.throws(() => inspectTraceFile(file('binary.json', Buffer.from([0, 1]))), { code: 'BINARY_INPUT' });
  assert.throws(() => inspectTraceFile(file('utf8.json', Buffer.from([0xc3, 0x28]))), { code: 'INVALID_JSON_OR_UTF8' });
});

test('CLI emits only metadata, pagination and safe errors', () => {
  const source = file('cli.json', JSON.stringify([message([tool('execute', { code: 'PRIVATE_CODE' }), tool('execute', { code: 'PRIVATE_CODE_2' })])]));
  const result = cli([source, '1', '1']);
  assert.equal(result.exit, 0);
  assert.equal(result.value.records[0].record, '0:1');
  assert(!JSON.stringify(result.value).includes('PRIVATE_CODE'));
  assert.equal(cli([]).value.error.code, 'INVALID_ARGUMENT');
  assert.equal(cli([source, 'NaN']).exit, 1);
  const malformed = cli([file('bad-cli.json', '{PRIVATE_ERROR_INPUT')]);
  assert.equal(malformed.exit, 1);
  assert.equal(malformed.value.error.code, 'INVALID_JSON_OR_UTF8');
  assert(!JSON.stringify(malformed.value).includes('PRIVATE'));
});

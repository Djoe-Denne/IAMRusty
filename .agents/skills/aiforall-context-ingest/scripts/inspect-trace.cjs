'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

function fail(code) {
  const error = new Error(code);
  error.code = code;
  throw error;
}

function integer(value, fallback, min, max) {
  const n = value === undefined ? fallback : value;
  if (!Number.isSafeInteger(n) || n < min || n > max) fail('INVALID_ARGUMENT');
  return n;
}

function identifier(value, fallback) {
  return typeof value === 'string' && /^[A-Za-z0-9_.:/-]{1,160}$/.test(value) ? value : fallback;
}

const sha = text => crypto.createHash('sha256').update(text).digest('hex');

function decodeTrace(trace) {
  if (Array.isArray(trace)) return { messages: trace, format: 'message-array', metadata: {} };
  if (!trace || typeof trace !== 'object') fail('UNSUPPORTED_TRACE');
  const keys = ['messages', 'data'].filter(key => Array.isArray(trace[key]));
  if (keys.length !== 1) fail('UNSUPPORTED_TRACE');
  return { messages: trace[keys[0]], format: keys[0] === 'messages' ? 'snapshot' : 'api-page', metadata: trace };
}

// Structural hints only, never a verdict about arbitrary nested code/data.
function errorMarkers(state) {
  const counts = { rejected: 0, isError: 0, scanLimited: false };
  let visited = 0;
  function visit(value, depth) {
    if (depth > 12 || ++visited > 20000) { counts.scanLimited = true; return; }
    if (!value || typeof value !== 'object') return;
    if (Array.isArray(value)) {
      for (const item of value) { visit(item, depth + 1); if (visited > 20000) break; }
      return;
    }
    if (value.status === 'rejected') counts.rejected++;
    if (value.isError === true) counts.isError++;
    for (const item of Object.values(value)) { visit(item, depth + 1); if (visited > 20000) break; }
  }
  for (const part of Array.isArray(state.content) ? state.content : []) {
    if (part?.type !== 'text' || typeof part.text !== 'string') continue;
    const text = part.text.trim();
    if (text.length > 2097152) { counts.scanLimited = true; continue; }
    if (!text.startsWith('[') && !text.startsWith('{')) continue;
    try { visit(JSON.parse(text), 0); } catch { /* Not structured execution evidence. */ }
  }
  return counts;
}

function executionInput(tool, state) {
  const input = state.input;
  if (!input || typeof input !== 'object') return null;
  if (tool === 'execute' && typeof input.code === 'string' && input.code.trim()) {
    return { kind: 'client-orchestration', language: 'javascript', code: input.code };
  }
  if (tool === 'shell' && typeof input.command === 'string' && input.command.trim()) {
    return { kind: 'shell-command', language: 'shell', code: input.command };
  }
  const contextTool = ['ctx_execute', 'ctx_execute_file'].some(name =>
    tool === name || tool.endsWith('.' + name) || tool.endsWith('_' + name));
  if (contextTool && typeof input.code === 'string' && input.code.trim()) {
    const languages = ['javascript', 'typescript', 'python', 'shell', 'ruby', 'go', 'rust', 'php', 'perl', 'r', 'elixir', 'csharp'];
    return { kind: 'context-script', language: languages.includes(input.language) ? input.language : 'unknown', code: input.code };
  }
  return null;
}

function outcome(kind, state) {
  if (state.status === 'error' || state.status === 'cancelled') return 'tool-error-reported';
  if (kind === 'client-orchestration') return 'unattributed';
  if (state.isError === true) return 'tool-error-reported';
  if (Number.isSafeInteger(state.exitCode)) return state.exitCode === 0 ? 'success-reported' : 'failure-reported';
  return state.status === 'completed' ? 'completed-unverified' : 'pending-or-unknown';
}

function inspectTrace(trace, options = {}) {
  if (!options || typeof options !== 'object' || Array.isArray(options)) fail('INVALID_ARGUMENT');
  const offset = integer(options.offset, 0, 0, 10000);
  const limit = integer(options.limit, 50, 1, 200);
  const { messages, format, metadata } = decodeTrace(trace);
  if (messages.length > 5000) fail('TOO_MANY_MESSAGES');
  const records = [];
  let toolCalls = 0, unsupportedMessageCount = 0;
  for (const [mi, message] of messages.entries()) {
    if (!message || typeof message !== 'object' || Array.isArray(message)) fail('INVALID_MESSAGE');
    if (typeof message.type !== 'string') { unsupportedMessageCount++; continue; }
    // Tool-shaped objects inside a user prompt are not executions.
    if (message.type !== 'assistant') continue;
    if (message.content !== undefined && !Array.isArray(message.content)) fail('INVALID_MESSAGE');
    for (const [ci, call] of (message.content || []).entries()) {
      if (call?.type !== 'tool') continue;
      if (++toolCalls > 10000) fail('TOO_MANY_TOOL_CALLS');
      const state = call.state && typeof call.state === 'object' ? call.state : {};
      const tool = identifier(call.name, 'unknown-tool');
      const input = executionInput(tool, state);
      if (!input) continue;
      records.push({
        record: mi + ':' + ci,
        messageID: identifier(message.id, 'unknown-message'),
        toolCallID: identifier(call.id, 'unknown-call'),
        tool, kind: input.kind, language: input.language,
        scriptSha256: sha(input.code), characters: input.code.length,
        outcome: outcome(input.kind, state),
        toolState: ['completed', 'error', 'cancelled', 'running', 'pending'].includes(state.status) ? state.status : 'unknown',
        explicitExitCode: Number.isSafeInteger(state.exitCode) ? state.exitCode : null,
        structuredErrorMarkers: errorMarkers(state),
        requiresManualReview: true,
      });
    }
  }
  const groups = new Map();
  for (const record of records) {
    const key = record.kind + ':' + record.language + ':' + record.scriptSha256;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(record.record);
  }
  const repeatedExactScripts = [...groups.entries()].filter(([, ids]) => ids.length > 1)
    .map(([key, ids]) => ({ kindAndLanguage: key.split(':').slice(0, 2).join(':'), scriptSha256: key.split(':').at(-1), count: ids.length, records: ids.slice(0, 20), recordsTruncated: ids.length > 20 }));
  const returned = records.slice(offset, offset + limit);
  return {
    schemaVersion: 1, inspectionOnly: true, format,
    messages: messages.length, toolCalls, unsupportedMessageCount,
    completeness: {
      declaredComplete: metadata.complete === true,
      pagesReported: Number.isSafeInteger(metadata.pages) && metadata.pages > 0 ? metadata.pages : null,
      nextCursorPresent: Boolean(metadata.cursor?.next),
      hasFinalIdle: messages.at(-1)?.type === 'idle',
    },
    totalScriptRecords: records.length, offset, returnedRecords: returned.length,
    truncatedRecords: offset + returned.length < records.length,
    records: returned, repeatedExactScripts,
    limits: ['Tool input is an attempted execution, not proof of useful success.', 'Client orchestration, aliases and loops require manual attribution.', 'Error markers can belong to returned data; review before interpreting.', 'No script text, prompt, result body or reasoning is emitted.'],
  };
}

function inspectTraceFile(file, options = {}) {
  if (typeof file !== 'string' || !path.isAbsolute(file)) fail('ABSOLUTE_PATH_REQUIRED');
  if (!options || typeof options !== 'object' || Array.isArray(options)) fail('INVALID_ARGUMENT');
  const maxBytes = integer(options.maxBytes, 16777216, 1, 134217728);
  const fd = fs.openSync(file, 'r');
  let bytes;
  try {
    const stat = fs.fstatSync(fd);
    if (!stat.isFile()) fail('NOT_A_FILE');
    if (stat.size > maxBytes) fail('INPUT_TOO_LARGE');
    const buffer = Buffer.alloc(Math.min(stat.size, maxBytes) + 1);
    let length = 0;
    while (length < buffer.length) {
      const n = fs.readSync(fd, buffer, length, buffer.length - length, null);
      if (!n) break;
      length += n;
    }
    if (length > maxBytes) fail('INPUT_TOO_LARGE');
    if (fs.fstatSync(fd).size !== length) fail('INPUT_CHANGED');
    bytes = buffer.subarray(0, length);
  } finally {
    fs.closeSync(fd);
  }
  if (bytes.includes(0)) fail('BINARY_INPUT');
  let trace;
  try { trace = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { fail('INVALID_JSON_OR_UTF8'); }
  const report = inspectTrace(trace, options);
  return { ...report, sourceSha256: sha(bytes), sourceBytes: bytes.length };
}

module.exports = { inspectTrace, inspectTraceFile };

if (require.main === module) {
  try {
    const [file, offsetText, limitText, ...extra] = process.argv.slice(2);
    if (!file || extra.length) fail('INVALID_ARGUMENT');
    const number = text => text === undefined ? undefined : /^\d+$/.test(text) ? Number(text) : NaN;
    console.log(JSON.stringify(inspectTraceFile(file, { offset: number(offsetText), limit: number(limitText) })));
  } catch (error) {
    const code = typeof error.code === 'string' && /^[A-Z0-9_]{1,64}$/.test(error.code) ? error.code : 'INSPECTION_FAILED';
    console.log(JSON.stringify({ ok: false, error: { code } }));
    process.exitCode = 1;
  }
}

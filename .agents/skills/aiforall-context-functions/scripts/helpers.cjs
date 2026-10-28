'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { compareOwnedContainerInventories } = require('./compare-inventories.cjs');
const { decodeEvidenceText } = require('./decode-evidence.cjs');

class AnalysisError extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

// Do not serialize Error: child_process exceptions can contain entire outputs.
function safeError(error) {
  const code = typeof error?.code === 'string' && /^[A-Z0-9_]{1,64}$/.test(error.code)
    ? error.code : 'OPERATION_FAILED';
  return { code };
}

function fail(code) {
  throw new AnalysisError(code);
}

function integer(value, fallback, min, max) {
  const number = value === undefined ? fallback : value;
  if (!Number.isSafeInteger(number) || number < min || number > max) fail('INVALID_ARGUMENT');
  return number;
}

function object(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail('INVALID_ARGUMENT');
}

function outside(root, target) {
  const relative = path.relative(root, target);
  return relative === '..' || relative.startsWith('..' + path.sep) || path.isAbsolute(relative);
}

function rootDirectory(root) {
  if (typeof root !== 'string' || !path.isAbsolute(root)) fail('INVALID_ROOT');
  const real = fs.realpathSync(root);
  if (!fs.statSync(real).isDirectory()) fail('INVALID_ROOT');
  return real;
}

function prepareFile(root, input, maxFileBytes) {
  object(input);
  if (typeof input.path !== 'string' || !input.path || input.path.includes('\0')) fail('INVALID_PATH');
  if (path.isAbsolute(input.path) || path.win32.isAbsolute(input.path)
      || /^[a-z]:/i.test(input.path)) fail('PATH_OUTSIDE_ROOT');
  const lexical = path.resolve(root, input.path);
  if (outside(root, lexical)) fail('PATH_OUTSIDE_ROOT');
  const real = fs.realpathSync(lexical);
  if (outside(root, real)) fail('PATH_OUTSIDE_ROOT');
  const stat = fs.statSync(real);
  if (!stat.isFile()) fail('NOT_A_FILE');
  if (stat.size > maxFileBytes) fail('FILE_TOO_LARGE');
  let ranges;
  if (input.ranges !== undefined) {
    if (!Array.isArray(input.ranges) || input.ranges.length < 1 || input.ranges.length > 100) fail('INVALID_RANGE');
    ranges = input.ranges.map(range => {
      object(range);
      const start = integer(range.start, undefined, 1, Number.MAX_SAFE_INTEGER);
      const end = integer(range.end, undefined, start, Number.MAX_SAFE_INTEGER);
      return { start, end };
    });
  }
  return { path: input.path, real, size: stat.size, ranges };
}

function readTextBounded(file, limit) {
  let fd;
  let bytesRead = 0;
  try {
    fd = fs.openSync(file, 'r');
    const stat = fs.fstatSync(fd);
    if (!stat.isFile()) fail('NOT_A_FILE');
    if (stat.size > limit) fail('FILE_TOO_LARGE');
    const buffer = Buffer.alloc(limit);
    while (bytesRead < buffer.length) {
      const count = fs.readSync(fd, buffer, bytesRead, buffer.length - bytesRead, null);
      if (count === 0) break;
      bytesRead += count;
    }
    if (fs.fstatSync(fd).size > bytesRead) fail('FILE_TOO_LARGE');
    const bytes = buffer.subarray(0, bytesRead);
    if (bytes.includes(0)) fail('BINARY_FILE');
    // Reject malformed UTF-8 rather than silently changing source evidence.
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return { text, bytesRead };
  } catch (error) {
    // Failed decoding/binary detection still consumed the shared read budget.
    return { error: safeError(error), bytesRead };
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

function linesOf(text) {
  if (!text) return [];
  const lines = text.split(/\r\n|\n|\r/);
  if (lines.at(-1) === '') lines.pop();
  return lines;
}

function selectedContent(lines, ranges, budget) {
  const requested = ranges || (lines.length ? [{ start: 1, end: lines.length }] : []);
  let content = '';
  let truncated = false;
  const returnedRanges = [];
  for (const range of requested) {
    if (range.start > lines.length) fail('RANGE_OUT_OF_BOUNDS');
    const end = Math.min(range.end, lines.length);
    let last = range.start - 1;
    for (let number = range.start; number <= end; number += 1) {
      const line = (content ? '\n' : '') + number + ': ' + lines[number - 1];
      if (content.length + line.length > budget) {
        // Keep only complete numbered lines so truncation cannot masquerade as evidence.
        truncated = true;
        break;
      }
      content += line;
      last = number;
    }
    if (last >= range.start) returnedRanges.push({ start: range.start, end: last });
    if (truncated) break;
  }
  return { content, truncated, returnedRanges };
}

function readEvidence(input) {
  object(input);
  const root = rootDirectory(input.root);
  if (!Array.isArray(input.files) || input.files.length < 1 || input.files.length > 50) fail('INVALID_ARGUMENT');
  const maxFileBytes = integer(input.maxFileBytes, 1048576, 1, 67108864);
  const maxTotalBytes = integer(input.maxTotalBytes, 8388608, 1, 67108864);
  const maxOutputChars = integer(input.maxOutputChars, 12000, 1, 2000000);
  // Preflight every selected path before the first read; retain each outcome.
  const prepared = input.files.map(entry => {
    try {
      return { prepared: prepareFile(root, entry, maxFileBytes) };
    } catch (error) {
      return { path: typeof entry?.path === 'string' ? entry.path : null, error: safeError(error) };
    }
  });
  let bytesRead = 0;
  let outputChars = 0;
  const entries = prepared.map(item => {
    if (item.error) return { path: item.path, status: 'error', error: item.error };
    const file = item.prepared;
    if (outputChars >= maxOutputChars) return { path: file.path, status: 'skipped', error: { code: 'OUTPUT_BUDGET' }, truncated: true };
    if (file.size > maxTotalBytes - bytesRead) return { path: file.path, status: 'skipped', error: { code: 'READ_BUDGET' }, truncated: true };
    try {
      const data = readTextBounded(file.real, Math.min(maxFileBytes, maxTotalBytes - bytesRead));
      bytesRead += data.bytesRead;
      if (data.error) return { path: file.path, status: 'error', error: data.error };
      const lines = linesOf(data.text);
      const selected = selectedContent(lines, file.ranges, maxOutputChars - outputChars);
      outputChars += selected.content.length;
      return { path: file.path, status: 'ok', lineCount: lines.length, ...selected };
    } catch (error) {
      return { path: file.path, status: 'error', error: safeError(error) };
    }
  });
  const successful = entries.filter(entry => entry.status === 'ok').length;
  const truncated = entries.some(entry => entry.truncated);
  return {
    ok: successful === entries.length && !truncated,
    partial: successful > 0 && (successful !== entries.length || truncated),
    truncated, bytesRead, outputChars, entries,
  };
}

function classifyCommandResult(result, options = {}) {
  object(result);
  object(options);
  const noMatch = options.noMatchExitCodes === undefined ? [] : options.noMatchExitCodes;
  if (!Array.isArray(noMatch) || noMatch.some(code => !Number.isSafeInteger(code) || code === 0)) fail('INVALID_ARGUMENT');
  const maxDiagnosticChars = integer(options.maxDiagnosticChars, 1000, 1, 10000);
  if (options.diagnosticText !== undefined && typeof options.diagnosticText !== 'string') fail('INVALID_ARGUMENT');
  if (result.status !== null && result.status !== undefined && !Number.isSafeInteger(result.status)) fail('INVALID_ARGUMENT');
  const exitCode = Number.isSafeInteger(result.status) ? result.status : null;
  const errorCode = result.error ? safeError(result.error).code : null;
  const signal = typeof result.signal === 'string' && /^[A-Z0-9]{1,32}$/.test(result.signal) ? result.signal : null;
  let kind;
  if (errorCode === 'ENOENT') kind = 'executable-not-found';
  else if (errorCode === 'ETIMEDOUT') kind = 'timeout';
  else if (errorCode === 'ENOBUFS' || errorCode === 'ERR_CHILD_PROCESS_STDIO_MAXBUFFER') kind = 'output-limit';
  else if (errorCode) kind = 'execution-error';
  else if (signal) kind = 'signal';
  else if (exitCode === 0) kind = 'success';
  else if (noMatch.includes(exitCode)) kind = 'no-match';
  else if (exitCode === null) kind = 'unknown';
  else kind = 'command-error';
  const classified = { ok: kind === 'success' || kind === 'no-match', kind, exitCode, signal, errorCode };
  if (options.diagnosticText !== undefined) {
    classified.diagnostic = options.diagnosticText.slice(0, maxDiagnosticChars);
    classified.diagnosticTruncated = options.diagnosticText.length > maxDiagnosticChars;
  }
  return classified;
}

async function collectIndependentReads(operations) {
  if (!Array.isArray(operations) || operations.length < 1 || operations.length > 100) fail('INVALID_ARGUMENT');
  const labels = new Set();
  for (const operation of operations) {
    if (typeof operation?.label !== 'string' || !operation.label.trim()
        || labels.has(operation.label) || typeof operation.run !== 'function') fail('INVALID_ARGUMENT');
    labels.add(operation.label);
  }
  const results = await Promise.allSettled(operations.map(async operation => operation.run()));
  return results.map((result, index) => result.status === 'fulfilled'
    ? { label: operations[index].label, status: result.status, value: result.value }
    : { label: operations[index].label, status: result.status, error: safeError(result.reason) });
}

module.exports = { readEvidence, classifyCommandResult, collectIndependentReads,
  compareOwnedContainerInventories, decodeEvidenceText };

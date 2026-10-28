'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { decodeEvidenceText: decode } = require('./helpers.cjs');
const le = text => Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(text, 'utf16le')]);
const be = text => {
  const b = Buffer.from(text, 'utf16le');
  for (let i = 0; i < b.length; i += 2) [b[i], b[i + 1]] = [b[i + 1], b[i]];
  return Buffer.concat([Buffer.from([0xfe, 0xff]), b]);
};

test('decodes UTF8 strictly without changing line endings', () => {
  const r = decode(Buffer.from('résultat\r\nline\n', 'utf8'));
  assert.equal(r.text, 'résultat\r\nline\n');
  assert.equal(r.encoding, 'utf-8');
  assert.equal(r.encodingSource, 'default-utf8');
  assert.equal(r.bomBytes, 0);
});
test('strips only one leading UTF8 BOM and preserves inner BOM', () => {
  const r = decode(Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from('\uFEFFx\uFEFF')]));
  assert.equal(r.text, '\uFEFFx\uFEFF');
  assert.equal(r.encodingSource, 'bom');
  assert.equal(r.bomBytes, 3);
});
test('decodes UTF16LE with BOM', () => {
  const r = decode(le('été 😀'));
  assert.equal(r.text, 'été 😀');
  assert.equal(r.encoding, 'utf-16le');
  assert.equal(r.bomBytes, 2);
});
test('decodes UTF16BE with BOM', () => {
  const r = decode(be('été 😀'));
  assert.equal(r.text, 'été 😀');
  assert.equal(r.encoding, 'utf-16be');
});
test('permits an explicit codec without BOM, no implicit UTF16 guessing', () => {
  const bytes = Buffer.from('text', 'utf16le');
  assert.throws(() => decode(bytes), { code: 'NUL_TEXT' });
  assert.equal(decode(bytes, { encoding: 'utf-16le' }).text, 'text');
});
test('rejects malformed UTF8 instead of replacement decoding', () => {
  assert.throws(() => decode(Buffer.from([0xc3, 0x28])), { code: 'INVALID_TEXT_ENCODING' });
});
test('rejects odd UTF16 and unpaired surrogate sequences', () => {
  assert.throws(() => decode(Buffer.from([0xff, 0xfe, 0x61])), { code: 'INVALID_TEXT_ENCODING' });
  assert.throws(() => decode(Buffer.from([0xff, 0xfe, 0, 0xd8])), { code: 'INVALID_TEXT_ENCODING' });
});
test('rejects contradictory explicit encoding and BOM', () => {
  assert.throws(() => decode(le('ok'), { encoding: 'utf-8' }), { code: 'ENCODING_CONFLICT' });
});
test('does not misread UTF32 BOMs as UTF16', () => {
  for (const b of [[0xff, 0xfe, 0, 0], [0, 0, 0xfe, 0xff]]) assert.throws(() => decode(Buffer.from(b)), { code: 'UNSUPPORTED_BOM' });
});
test('NUL is rejected unless explicitly allowed, never stripped', () => {
  assert.throws(() => decode(Buffer.from('a\0b')), { code: 'NUL_TEXT' });
  assert.equal(decode(Buffer.from('a\0b'), { allowNul: true }).text, 'a\0b');
});
test('honors byte bounds and view offsets without mutating input', () => {
  const bytes = Buffer.from('!hello!');
  const copy = Buffer.from(bytes);
  const r = decode(bytes.subarray(1, 6), { maxBytes: 5 });
  assert.equal(r.text, 'hello');
  assert.equal(r.byteLength, 5);
  assert.deepEqual(bytes, copy);
  assert.throws(() => decode(bytes, { maxBytes: 6 }), { code: 'INPUT_TOO_LARGE' });
});
test('empty input is valid text', () => assert.equal(decode(new Uint8Array()).text, ''));
test('invalid options and non-byte inputs have stable sanitized errors', () => {
  assert.throws(() => decode('private input'), { code: 'INPUT_BYTES_REQUIRED', message: 'INPUT_BYTES_REQUIRED' });
  assert.throws(() => decode(Buffer.from('x'), null), { code: 'INVALID_OPTIONS' });
  assert.throws(() => decode(Buffer.from('x'), { encoding: 'latin1' }), { code: 'UNSUPPORTED_ENCODING' });
  assert.throws(() => decode(Buffer.from('x'), { allowNul: 'true' }), { code: 'INVALID_OPTIONS' });
  for (const maxBytes of [0, -1, 1.5, 16777217]) assert.throws(() => decode(Buffer.from('x'), { maxBytes }), { code: 'INVALID_LIMIT' });
});

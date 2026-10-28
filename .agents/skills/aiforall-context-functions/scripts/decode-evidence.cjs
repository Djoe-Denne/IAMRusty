'use strict';

function fail(code) { const error = new Error(code); error.code = code; throw error; }

/** Strict, bounded decoding only. No filesystem access, fallback codec or log interpretation. */
function decodeEvidenceText(bytes, options = {}) {
  if (!(bytes instanceof Uint8Array)) fail('INPUT_BYTES_REQUIRED');
  if (!options || typeof options !== 'object' || Array.isArray(options)) fail('INVALID_OPTIONS');
  const maxBytes = options.maxBytes === undefined ? 1048576 : options.maxBytes;
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 16777216) fail('INVALID_LIMIT');
  if (bytes.byteLength > maxBytes) fail('INPUT_TOO_LARGE');
  const requested = options.encoding === undefined ? 'auto' : options.encoding;
  if (!['auto', 'utf-8', 'utf-16le', 'utf-16be'].includes(requested)) fail('UNSUPPORTED_ENCODING');
  if (options.allowNul !== undefined && typeof options.allowNul !== 'boolean') fail('INVALID_OPTIONS');
  if (bytes.length >= 4 && ((bytes[0] === 0xff && bytes[1] === 0xfe && bytes[2] === 0 && bytes[3] === 0)
    || (bytes[0] === 0 && bytes[1] === 0 && bytes[2] === 0xfe && bytes[3] === 0xff))) fail('UNSUPPORTED_BOM');
  let bomEncoding;
  let bomBytes = 0;
  if (bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    bomEncoding = 'utf-8'; bomBytes = 3;
  } else if (bytes.length >= 2 && bytes[0] === 0xff && bytes[1] === 0xfe) {
    bomEncoding = 'utf-16le'; bomBytes = 2;
  } else if (bytes.length >= 2 && bytes[0] === 0xfe && bytes[1] === 0xff) {
    bomEncoding = 'utf-16be'; bomBytes = 2;
  }
  if (requested !== 'auto' && bomEncoding && requested !== bomEncoding) fail('ENCODING_CONFLICT');
  const encoding = bomEncoding || (requested === 'auto' ? 'utf-8' : requested);
  const body = bytes.subarray(bomBytes);
  if (encoding !== 'utf-8' && body.length % 2 !== 0) fail('INVALID_TEXT_ENCODING');
  let text;
  try {
    // Strip exactly the identified encoding BOM once, preserving inner U+FEFF.
    text = new TextDecoder(encoding, { fatal: true, ignoreBOM: true }).decode(body);
  } catch {
    fail('INVALID_TEXT_ENCODING');
  }
  if (!options.allowNul && text.includes('\0')) fail('NUL_TEXT');
  return { encoding, encodingSource: bomEncoding ? 'bom' : requested === 'auto' ? 'default-utf8' : 'explicit',
    bomBytes, byteLength: bytes.byteLength, text };
}

module.exports = { decodeEvidenceText };

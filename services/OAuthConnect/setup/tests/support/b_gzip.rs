//! Fixed tiny wire fixture: valid KV-v2 JSON, not compressed junk or a test decompressor.

// Generated once with node:zlib.gzipSync. The uncompressed representation is exactly
// {"data":{"data":{"client_secret":"vault-client-sentinel"}}} (59 UTF-8 bytes).
// Vendor gzip-positive and SDK gzip-negative use the same known-valid wire bytes.
/// Tiny gzip member with a valid, known KV-v2 JSON representation.
pub const GZIP_KV: &[u8] = &[
    31, 139, 8, 0, 0, 0, 0, 0, 0, 10, 171, 86, 74, 73, 44, 73, 84, 178, 170, 134, 211, 201, 57,
    153, 169, 121, 37, 241, 197, 169, 201, 69, 169, 37, 74, 86, 74, 101, 137, 165, 57, 37, 186, 16,
    81, 221, 226, 212, 188, 146, 204, 188, 212, 28, 165, 218, 218, 90, 0, 73, 253, 247, 41, 59, 0,
    0, 0,
];

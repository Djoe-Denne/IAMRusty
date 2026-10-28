// Independent test-vector generator. Never imports the Rust verifier/codec.
// The ephemeral RSA private keys exist only in this process, never in an output.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const source = path.resolve(__dirname, '../../../../crates/apparatus-contracts/tests/fixtures/runtime_admission_attestation_v1.json');
const claims = JSON.parse(fs.readFileSync(source, 'utf8')).claims;
function jcs(value) {
    if (Array.isArray(value)) return '[' + value.map(jcs).join(',') + ']';
    if (value && typeof value === 'object') return '{' + Object.keys(value).sort().map(key => JSON.stringify(key) + ':' + jcs(value[key])).join(',') + '}';
    return JSON.stringify(value);
}
const b64 = bytes => Buffer.from(bytes).toString('base64url');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest();
const { privateKey, publicKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 3072, publicExponent: 65537 });
const publicJwk = publicKey.export({ format: 'jwk' });
const kid = 'ara-v1-' + b64(sha(jcs({ e: publicJwk.e, kty: 'RSA', n: publicJwk.n })));
const header = { alg: 'RS256', typ: 'apparatus-admission-materialization+jwt', kid };
function signed(payload = claims, h = header, raw) {
    const message = b64(jcs(h)) + '.' + b64(raw === undefined ? jcs(payload) : raw);
    return message + '.' + b64(crypto.sign('RSA-SHA256', Buffer.from(message), { key: privateKey, padding: crypto.constants.RSA_PKCS1_PADDING }));
}
const cases = [];
function add(name, compact, expected, now = '1700000000000', expectedChallenge = claims.challenge) {
    cases.push({ name, compact_jws: compact, expected, db_now_unix_ms: now,
        expected_challenge: expectedChallenge,
        expected_fingerprint_hex: sha(compact).toString('hex') });
}
const nominal = signed();
add('prepare_independent_rs256', nominal, 'valid');
add('last_millisecond_before_proof_expiry', nominal, 'valid', '1700000004999');
add('exact_proof_expiry_no_grace', nominal, 'RUNTIME_INVALID_PROOF', '1700000005000');
add('future_observation', nominal, 'RUNTIME_INVALID_PROOF', '1699999999999');
add('exact_challenge_expiry', nominal, 'RUNTIME_INVALID_PROOF', '1700000060000');
const open = structuredClone(claims);
open.challenge.action = 'open';
open.challenge.challenge_id = '00000000-0000-4000-8000-000000000035';
open.jti = open.challenge.challenge_id;
open.challenge.service_uid = 'admitted-service-open-uid';
open.challenge.revision_uid = 'admitted-revision-open-uid';
add('open_new_challenge_independent_rs256', signed(open), 'valid', '1700000000000', open.challenge);
add('prepare_proof_cannot_serve_open', nominal, 'RUNTIME_INVALID_PROOF', '1700000000000', open.challenge);
const signatureParts = nominal.split('.');
const badSignature = Buffer.from(signatureParts[2], 'base64url');
badSignature[0] ^= 1;
add('signature_bit_flipped', signatureParts.slice(0, 2).join('.') + '.' + b64(badSignature), 'RUNTIME_INVALID_PROOF');
for (const [field, replacement] of [['alg', 'none'], ['alg', 'HS256'], ['typ', 'JWT'], ['kid', 'ara-v1-unapproved-key'], ['jku', 'https://untrusted.invalid/keys'], ['jwk', { kty: 'RSA' }], ['x5u', 'https://untrusted.invalid/cert'], ['crit', ['unknown']]]) {
    const changed = { ...header, [field]: replacement };
    add('header_' + field + '_' + cases.length, signed(claims, changed), 'RUNTIME_INVALID_PROOF');
}
for (const [field, replacement] of [['iss', 'urn:untrusted:issuer'], ['aud', 'urn:untrusted:audience'], ['purpose', 'apparatus.runtime.invoke.v1'], ['jti', '00000000-0000-4000-8000-000000000039'], ['schema_version', 2], ['admission_revision', '0']]) {
    add('claims_' + field, signed({ ...claims, [field]: replacement }), 'RUNTIME_INVALID_PROOF');
}
for (const field of ['binding_id', 'project_id', 'migration_id', 'authority_id', 'installation_id', 'challenge_id']) {
    const changed = structuredClone(claims);
    changed.challenge[field] = '00000000-0000-4000-8000-000000000039';
    add('substituted_challenge_' + field, signed(changed), 'RUNTIME_INVALID_PROOF');
}
for (const field of ['generation', 'revocation_epoch', 'projection_version', 'grant_revision']) {
    const changed = structuredClone(claims);
    changed.challenge.expected[field] = '999';
    add('substituted_cas_' + field, signed(changed), 'RUNTIME_INVALID_PROOF');
}
add('valid_signature_noncanonical_json', signed(claims, header, JSON.stringify(claims, null, 2)), 'RUNTIME_INVALID_PROOF');
const duplicate = jcs(claims).replace('"policy_id":"apparatus-p4-conformance/v1"', '"policy_id":"apparatus-p4-conformance/v1","policy_id":"untrusted-policy"');
add('valid_signature_duplicate_policy', signed(claims, header, duplicate), 'RUNTIME_INVALID_PROOF');
add('unapproved_policy', signed({ ...claims, policy_id: 'untrusted-policy/v1' }), 'RUNTIME_RELEASE_NOT_ADMITTED');
add('tagged_cri_not_digest_pinned', signed({ ...claims, cri_image: 'registry.test/apparatus/reference-kv:latest' }), 'RUNTIME_RELEASE_NOT_ADMITTED');
add('unapproved_envelope_repository', signed({ ...claims, envelope_reference: 'foreign.test/envelope@sha256:' + 'f'.repeat(64) }), 'RUNTIME_RELEASE_NOT_ADMITTED');
const { publicKey: weakPublicKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048, publicExponent: 65537 });
const fixture = {
    gaps: ['G-3', 'G-7'], contract: 'apparatus-knative-runtime §13.5',
    provenance: 'Node crypto RSA-SHA256/PKCS1-v1_5; independent ECMAScript canonical serialization; generated test keys, PRIVATE KEYS NEVER SAVED',
    authority_id: claims.challenge.authority_id, installation_id: claims.challenge.installation_id,
    public_jwk: publicJwk, weak_public_jwk: weakPublicKey.export({ format: 'jwk' }), expected_kid: kid,
    allowed_repositories: ['registry.test/apparatus/envelopes', 'registry.test/apparatus/reference-kv'], cases,
};
const output = path.join(__dirname, 'runtime_admission_jws_v1.json');
fs.writeFileSync(output, JSON.stringify(fixture, null, 2) + '\n');
console.log(JSON.stringify({ output, cases: cases.length, rsa_bits: 3072, private_key_files: 0 }));

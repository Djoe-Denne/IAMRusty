import http from 'k6/http';
import { check, fail } from 'k6';
import { Counter } from 'k6/metrics';

// Windows runner supplies public runtime metadata, never passwords or tokens.
const campaign = JSON.parse(open(__ENV.IAM_CAMPAIGN_FILE));
if (campaign.version !== 1 || !/^[a-f0-9]{32}$/.test(campaign.run_id)
    || !/^https:\/\/127\.0\.0\.1:[0-9]+$/.test(campaign.origin)) {
  throw new Error('IAM_CAMPAIGN_INVALID');
}
const finished = new Counter('iam_finished');
const preflight = __ENV.IAM_PHASE === 'preflight';
if (!preflight && __ENV.IAM_PHASE !== 'functional') throw new Error('IAM_PHASE_INVALID');
const expected = preflight ? 1 : 12;
export const options = {
  scenarios: { iam: { executor: 'shared-iterations', vus: 1, iterations: 1, maxDuration: '120s', gracefulStop: '2s' } },
  thresholds: { checks: ['rate==1'], iam_finished: ['count==1'], http_reqs: [`count==${expected}`] },
  maxRedirects: 0, insecureSkipTLSVerify: false,
  systemTags: ['name', 'method', 'status', 'check', 'scenario'],
};

function request(name, path, body = null, headers = {}) {
  const params = {
    headers: { 'Content-Type': 'application/json', ...headers },
    redirects: 0, timeout: '8s', tags: { name },
  };
  return body === null
    ? http.get(campaign.origin + '/iam' + path, params)
    : http.post(campaign.origin + '/iam' + path, JSON.stringify(body), params);
}
function json(response) { try { return response.json(); } catch { return {}; } }
function expect(name, response, status, predicate = () => true) {
  if (!check(response, { [name]: (r) => r.status === status && predicate(r) })) {
    fail(`IAM_CHECK_FAILED:${name}`); // Never include a raw response or exception.
  }
}
const noSession = (response) => {
  const b = json(response);
  return !b.access_token && !b.refresh_token && !b.registration_token && !b.user;
};

export default function () {
  expect('health', request('health', '/health'), 200, (r) => r.body === 'OK');
  if (preflight) { finished.add(1); return; }
  expect('jwks-public-only', request('jwks-public-only', '/.well-known/jwks.json'), 200, (r) => {
    const keys = json(r).keys;
    return Array.isArray(keys) && keys.length > 0 && keys.some((k) => k.kty === 'RSA' && k.n && k.e)
      && keys.every((k) => !['d', 'p', 'q', 'dp', 'dq', 'qi', 'oth', 'k'].some((field) => field in k));
  });
  expect('username-available', request('username-available', `/api/auth/username/check?username=k6_${campaign.run_id}`),
    200, (r) => json(r).available === true && Array.isArray(json(r).suggestions));
  const email = `k6_${campaign.run_id}@example.invalid`;
  expect('signup-invalid-email', request('signup-invalid-email', '/api/auth/signup',
    { email: 'not-an-email', password: 'InvalidFixture1!' }), 422, noSession);
  expect('signup-weak-password', request('signup-weak-password', '/api/auth/signup',
    { email, password: 'short' }), 422, noSession);
  expect('signup-missing-password', request('signup-missing-password', '/api/auth/signup', { email }), 422, noSession);
  expect('registration-invalid-token', request('registration-invalid-token', '/api/auth/complete-registration',
    { registration_token: 'not-a-jwt', username: `k6_${campaign.run_id}` }), 400, noSession);
  expect('refresh-missing-token', request('refresh-missing-token', '/api/token/refresh', {}), 422, noSession);
  expect('refresh-unknown-token', request('refresh-unknown-token', '/api/token/refresh',
    { refresh_token: `k6-nonexistent-${campaign.run_id}` }), 401, noSession);
  // Exact edge decisions: ext-authz::deny is 403; direct IAM middleware is different.
  expect('me-no-token', request('me-no-token', '/api/me'), 403, noSession);
  expect('me-malformed-token', request('me-malformed-token', '/api/me', null,
    { Authorization: 'Bearer not-a-jwt' }), 403, noSession);
  expect('me-forged-principal', request('me-forged-principal', '/api/me', null,
    { 'x-principal-sub': '00000000-0000-0000-0000-000000000001', 'x-principal-iss': 'forged' }), 403, noSession);
  finished.add(1);
}

export function handleSummary(data) {
  const checks = [];
  function collect(group) {
    for (const c of group.checks || []) checks.push({ name: c.name, passes: c.passes, fails: c.fails });
    for (const child of group.groups || []) collect(child);
  }
  collect(data.root_group);
  return { [__ENV.IAM_SUMMARY_FILE]: JSON.stringify({
    version: 1, run_id: campaign.run_id, phase: __ENV.IAM_PHASE, expected, checks,
    finished: data.metrics.iam_finished?.values.count || 0,
    requests: data.metrics.http_reqs?.values.count || 0,
  }, null, 2) };
}

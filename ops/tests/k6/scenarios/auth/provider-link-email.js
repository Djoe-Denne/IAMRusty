import http from 'k6/http';
import { campaignTarget } from '../../lib/campaign-target.js';
import { scenarioContract } from '../../lib/case-contract.js';

const CASE = 'auth-provider-link-email';
const contract = scenarioContract(CASE);
const { origin, provider, cookieName, run_id: run } = campaignTarget();
const iam = `${origin}/iam/api/auth`;
const profiles = [
  { kind: 'attested', verified: true },
  { kind: 'false', verified: false },
  { kind: 'omitted', verified: false },
];
const uuid = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/;

export const options = {
  scenarios: { [CASE]: { executor: 'per-vu-iterations', vus: 1, iterations: 1,
    maxDuration: '70s', gracefulStop: '2s' } },
  setupTimeout: '100s',
  maxRedirects: 0,
  discardResponseBodies: true,
  thresholds: contract.thresholds,
  systemTags: ['name', 'check', 'scenario', 'group'],
};

function request(name, headers = {}, responseType = 'none') {
  return { headers, responseType, redirects: 0, timeout: '7s', tags: { name } };
}

function json(response) {
  try { return response.json(); } catch { return null; }
}

function record(name, condition) {
  return contract.check(Boolean(condition), { [name]: (passed) => passed });
}

function requireCheck(name, condition) {
  if (!record(name, condition)) throw new Error('EXPLICIT_LINK_PREREQUISITE_FAILED');
}

function queryAt(url, exact) {
  if (typeof url !== 'string' || !url.startsWith(exact + '?') || url.includes('#')) return null;
  const result = Object.create(null);
  try {
    for (const field of url.slice(exact.length + 1).split('&')) {
      const equal = field.indexOf('=');
      if (equal < 1) return null;
      const key = decodeURIComponent(field.slice(0, equal).replace(/\+/g, ' '));
      if (Object.prototype.hasOwnProperty.call(result, key)) return null;
      result[key] = decodeURIComponent(field.slice(equal + 1).replace(/\+/g, ' '));
    }
  } catch { return null; }
  return result;
}

function authorization(providerId, action, choice, label, bearer) {
  const callback = `${iam}/${providerId}/callback`;
  const jar = http.cookieJar();
  jar.clear(origin);
  const start = http.get(`${iam}/${providerId}/${action}`,
    request(`${action}-start`, bearer ? { Authorization: `Bearer ${bearer}` } : {}));
  const location = start.headers.Location;
  const query = queryAt(location, provider);
  const nonce = jar.cookiesForURL(callback)[cookieName];
  requireCheck(`${label}-start`, start.status === 303 && query?.redirect_uri === callback
    && query.response_type === 'code' && /^[a-f0-9]{64}$/.test(query.client_id)
    && query.state && query.code_challenge_method === 'S256' && /^[A-Za-z0-9_-]{43}$/.test(query.code_challenge)
    && !Object.prototype.hasOwnProperty.call(query, 'fixture_account')
    && nonce?.length === 1 && /^[A-Za-z0-9_-]{43}$/.test(nonce[0])
    && Object.keys(jar.cookiesForURL(provider)).length === 0);

  // The bounded choice belongs only to the disposable external IdP. IAM's
  // access token stays on the LINK start; its cookie is host-only.
  const approved = http.get(`${location}&fixture_account=${choice}`, request('idp-authorize'));
  const returned = approved.headers.Location;
  const fields = queryAt(returned, callback);
  requireCheck(`${label}-redirect`, approved.status === 303 && fields?.state === query.state
    && /^[A-Za-z0-9_-]{43}$/.test(fields.code) && Object.keys(fields).length === 2);
  return http.get(returned, request(`${action}-callback`, {}, 'text'));
}

export function setup() {
  if (!__ENV.K6_CAMPAIGN_FILE || __ENV.K6_ORIGIN !== origin || __ENV.K6_RUN_ID !== run
      || !__ENV.K6_SUMMARY_PATH) throw new Error('EXPLICIT_LINK_INPUT_INVALID');
  const observations = http.get(provider.replace(/\/oauth\/authorize$/, '/observations'),
    request('hosted-link-fixture', {}, 'text'));
  const fixture = json(observations);
  requireCheck('hosted-link-fixture', observations.status === 200
    && fixture?.run_id === run && fixture.fixture_profile === 'hosted-link-email');
  const denied = http.get(`${iam}/github/link`, request('unauthenticated-link-start'));
  requireCheck('link-start-requires-authentication', denied.status === 403
    && denied.headers.Location === undefined && denied.headers['Set-Cookie'] === undefined);

  // There is no account-deletion API. The admitted disposable campaign owns
  // all partial writes, including a failure in setup before teardown can run.
  return profiles.map(({ kind }) => {
    const choice = `owner-${kind}`;
    const username = `s502-${kind}-${run}`;
    const primary = `${run}-${choice}@example.invalid`;
    const callback = authorization('gitlab', 'login', choice, `${kind}-owner`);
    const registration = json(callback);
    requireCheck(`${kind}-registration-required`, callback.status === 202
      && registration?.operation === 'registration_required' && registration.provider_info?.email === primary
      && typeof registration.registration_token === 'string' && registration.registration_token.length > 0);
    const response = http.post(`${iam}/complete-registration`, JSON.stringify({
      registration_token: registration.registration_token, username,
    }), request('owner-complete-registration', { 'Content-Type': 'application/json' }, 'text'));
    const account = json(response);
    requireCheck(`${kind}-owner-registered`, response.status === 200 && uuid.test(account?.user?.id)
      && account.user.username === username && account.user.email === primary
      && /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(account.access_token));
    return { id: account.user.id, token: account.access_token, primary, username };
  });
}

export default function (owners) {
  contract.start();
  record('distinct-link-owners', new Set(owners.map(({ id }) => id)).size === profiles.length);
  for (const [index, { kind, verified }] of profiles.entries()) {
    const owner = owners[index];
    const choice = `link-${kind}`;
    const secondary = `${run}-${choice}@example.invalid`;
    const response = authorization('github', 'link', choice, `${kind}-link`, owner.token);
    const result = json(response);
    record(`${kind}-explicit-link`, response.status === 200 && result?.operation === 'link'
      && result.user?.id === owner.id && result.user?.username === owner.username);
    const emails = Array.isArray(result?.emails) ? result.emails : [];
    const first = emails.filter((row) => row?.is_primary === true);
    const added = emails.filter((row) => row?.email === secondary);
    record(`${kind}-primary-preserved`, result?.user?.email === owner.primary && first.length === 1
      && first[0].email === owner.primary && first[0].is_verified === true && uuid.test(first[0].id));
    record(`${kind}-new-secondary`, result?.new_email_added === true && result.new_email === secondary
      && emails.length === 2 && added.length === 1 && added[0].is_primary === false
      && uuid.test(added[0].id) && added[0].id !== first[0]?.id);
    record(`${kind}-secondary-verification`, added.length === 1 && added[0].is_verified === verified);
  }
  contract.finish();
}

export function handleSummary(data) { return contract.summary(data); }

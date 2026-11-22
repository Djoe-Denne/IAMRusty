import { check } from 'k6';
import http from 'k6/http';
import { campaignTarget } from '../../lib/campaign-target.js';
import { started, completed, project } from '../../lib/summary.js';

const target = campaignTarget('iam-link-conflicts');
const phase = __ENV.K6_LINK_PHASE;
const receiptPath = __ENV.K6_LINK_RECEIPT;
const summaryPath = __ENV.K6_SUMMARY_PATH;
const { origin, provider, run_id: runId, cookieName } = target;
const callback = origin + '/iam/api/auth/gitlab/callback';
const registration = origin + '/iam/api/auth/complete-registration';
const setupPhase = phase === 'setup';

if (!['setup', 'email', 'identity'].includes(phase)
    || __ENV.K6_RUN_ID !== runId || __ENV.K6_ORIGIN !== origin
    || !/^[A-Za-z]:[\\/].+[\\/]link-setup-private\.json$/.test(receiptPath ?? '')
    || !summaryPath || receiptPath === summaryPath) throw new Error('LINK_INPUT_INVALID');

const caseId = 'auth-link-conflict-' + phase;
export const options = {
  scenarios: { [caseId]: {
    executor: 'per-vu-iterations', vus: 1, iterations: 1,
    maxDuration: '30s', gracefulStop: '2s',
  } },
  maxRedirects: 0,
  discardResponseBodies: true,
  thresholds: {
    checks: ['rate==1'], case_started: ['count==1'], case_completed: ['count==1'],
    http_reqs: [setupPhase ? 'count==8' : 'count==3'],
  },
  systemTags: ['name', 'check', 'scenario', 'group'],
};

function validReceipt(value) {
  return value && Object.keys(value).sort().join(',') === 'access_token,run_id,version'
    && value.version === 1 && value.run_id === runId
    && typeof value.access_token === 'string' && value.access_token.length <= 8192
    && /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(value.access_token);
}

let receipt;
if (!setupPhase) {
  try {
    receipt = JSON.parse(open(receiptPath));
    if (!validReceipt(receipt)) throw new Error();
  } catch { throw new Error('LINK_RECEIPT_INVALID'); }
}

function json(response) {
  try { return response.json(); } catch { return null; }
}

function queryAt(url, base) {
  if (typeof url !== 'string' || !url.startsWith(base + '?') || url.includes('#')) return null;
  const params = Object.create(null);
  try {
    for (const field of url.slice(base.length + 1).split('&')) {
      const equal = field.indexOf('=');
      if (equal < 1) return null;
      const key = decodeURIComponent(field.slice(0, equal).replace(/\+/g, ' '));
      if (Object.prototype.hasOwnProperty.call(params, key)) return null;
      params[key] = decodeURIComponent(field.slice(equal + 1).replace(/\+/g, ' '));
    }
  } catch { return null; }
  return params;
}

function header(response, name) {
  const key = Object.keys(response.headers).find((item) => item.toLowerCase() === name);
  return key === undefined ? undefined : response.headers[key];
}

function record(name, outcome) {
  return check(Boolean(outcome), { [name]: (value) => value });
}

function requireCheck(name, outcome) {
  if (!record(name, outcome)) throw new Error('LINK_FLOW_FAILED');
}

function request(name, headers = {}, responseType = 'none') {
  return { redirects: 0, timeout: '10s', responseType, headers, tags: { name } };
}

function matchesError(value, status, code, message) {
  return value && Object.keys(value).join(',') === 'error'
    && value.error && Object.keys(value.error).sort().join(',') === 'error_code,message,status'
    && value.error.status === status && value.error.error_code === code && value.error.message === message;
}

function authorize(principal, bearer, startCheck, redirectCheck) {
  const jar = http.cookieJar();
  jar.clear(origin);
  const mode = bearer ? 'link' : 'login';
  const begin = http.get(`${origin}/iam/api/auth/gitlab/${mode}`,
    request(mode + '-start', bearer ? { Authorization: 'Bearer ' + bearer } : {}));
  const destination = header(begin, 'location');
  const authorizeQuery = queryAt(destination, provider);
  const nonce = jar.cookiesForURL(callback)[cookieName];
  requireCheck(startCheck, begin.status === 303 && authorizeQuery?.response_type === 'code'
    && authorizeQuery.redirect_uri === callback && authorizeQuery.client_id === 'native-' + runId
    && authorizeQuery.state && !Object.prototype.hasOwnProperty.call(authorizeQuery, 'fixture_account')
    && authorizeQuery.code_challenge_method === 'S256'
    && /^[A-Za-z0-9_-]{43}$/.test(authorizeQuery.code_challenge ?? '')
    && nonce?.length === 1 && nonce[0]);

  // The additional selector is test fixture input only, never sent to IAM or
  // any other provider. Keep IAM credentials exclusively on the LINK start.
  const fixture = http.get(destination + '&fixture_account=' + principal, request('fixture-authorize'));
  const returned = header(fixture, 'location');
  const result = queryAt(returned, callback);
  requireCheck(redirectCheck, fixture.status === 303 && result?.state === authorizeQuery.state
    && typeof result.code === 'string' && result.code.length > 0);
  return http.get(returned, request('iam-callback', {}, 'text'));
}

function register(principal, prefix) {
  const response = authorize(principal, null, prefix + '-login-start', prefix + '-provider-redirect');
  const data = json(response);
  requireCheck(prefix + '-registration-required', response.status === 202
    && data?.operation === 'registration_required'
    && typeof data.registration_token === 'string' && data.registration_token.length > 0);
  const username = 'lc' + principal + runId.slice(0, 16);
  const result = http.post(registration, JSON.stringify({
    registration_token: data.registration_token, username,
  }), request('complete-registration', { 'Content-Type': 'application/json' }, 'text'));
  const account = json(result);
  requireCheck(prefix + '-registration-completed', result.status === 200 && account?.user?.id
    && account.user.username === username && account.user.email === `${runId}-${principal}@example.invalid`);
  return account;
}

export function setup() {
  if (!setupPhase) return null;
  const owner = register('a', 'owner');
  const actor = register('b', 'target');
  const output = { version: 1, run_id: runId, access_token: actor.access_token };
  requireCheck('distinct-accounts', owner.user.id !== actor.user.id && validReceipt(output));
  return output;
}

export default function () {
  started.add(1);
  if (!setupPhase) {
    const email = phase === 'email';
    const response = authorize(email ? 'c' : 'a', receipt.access_token, 'link-start', 'provider-redirect');
    const value = json(response);
    const errorCode = email ? 'provider_email_conflict' : 'provider_already_linked';
    const message = email ? 'Provider email address cannot be used'
      : 'This gitlab account is already linked to another user';
    const fixed = response.status === 409 && matchesError(value, 409, errorCode, message);
    const baseline = email
      ? response.status === 409 && matchesError(value, 409, 'provider_already_linked',
        'This gitlab account is already linked to another user')
      : response.status === 500 && matchesError(value, 500, 'link_failed', 'Link provider failed');
    // A matching response is evidence of its wire shape, not a unique cause:
    // the old identity response also occurs for unrelated generic errors.
    record('conflict-recognized-response', fixed || baseline);
    record('conflict-http-status', response.status === 409);
    record('conflict-error-code', value?.error?.error_code === errorCode);
    record('conflict-envelope', matchesError(value, 409, errorCode, message));
  }
  completed.add(1);
}

export function handleSummary(data) {
  const result = project(data, 'HTTP', caseId);
  if (setupPhase && validReceipt(data.setup_data)) {
    // The parent supplies an owner-only campaign path. Never export the raw
    // legacy summary or put the receipt in the public projection.
    result[receiptPath] = JSON.stringify(data.setup_data);
  }
  return result;
}

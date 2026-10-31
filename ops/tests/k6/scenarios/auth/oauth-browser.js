import { browser } from 'k6/browser';
import { check } from 'k6';
import encoding from 'k6/encoding';
import { started, completed, project } from '../../lib/summary.js';

// k6 2.3.0 only: page.on(request/response), allHeaders(), context.cookies(),
// page.evaluate() and browser.version(); no Playwright API or Node Buffer.
// Parent admission/loaded-config and same-run IAM/IdP/PKCE correlation remain
// mandatory. This browser result alone is NOT a campaign PASS (0412 stays IT).
// Planned parent deltas: IAM +7, ext-authz +4, authorize/token/profile/PKCE +3
// each, only after confirming loaded routes and excluding concurrent traffic.
// Closed checks: every named expect() below must have passes=1, fails=0 (26).
// browser-version, registration-callback, registration-required,
// registration-nonce, registration-completed, login-callback, login-identity,
// login-authority, login-nonce, relink-start, relink-callback, relink-identity,
// relink-nonce, negative-link, negative-relink-start, negative-relink-alias,
// oauth-observations, oauth-iam-cardinality, login-redirects,
// oauth-cookie-attributes, oauth-callback-headers, registration-request-headers,
// relink-start-authorization, provider-authorizations, provider-no-credentials,
// negative-headers.
// No raw body, URL, header, token, nonce or exception is exported by project().
const ORIGIN = 'http://127.0.0.1:18080';
const PROVIDER = 'http://localhost:18081/oauth/authorize';
const LOGIN = '/iam/api/auth/gitlab/login';
const CALLBACK = '/iam/api/auth/gitlab/callback';
const REGISTER = '/iam/api/auth/complete-registration';
const START = '/iam/api/auth/gitlab/relink-start';
const RELINK = '/iam/api/auth/gitlab/relink-callback';
const NEGATIVES = [
  ['/iam/api/auth/gitlab/link', 'negative-link'],
  [START, 'negative-relink-start'],
  ['/iam/api/auth/gitlab/relink/callback', 'negative-relink-alias'],
];
const PATHS = [LOGIN, CALLBACK, REGISTER, START, RELINK, ...NEGATIVES.map(([path]) => path)];

export const options = {
  scenarios: {
    'auth-oauth-browser': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '90s', gracefulStop: '3s', options: { browser: { type: 'chromium' } },
    },
  },
  thresholds: { checks: ['rate==1'], case_started: ['count==1'], case_completed: ['count==1'] },
  systemTags: ['name', 'check', 'scenario'],
};

function expect(name, condition) {
  if (!check(Boolean(condition), { [name]: (value) => value })) throw new Error('AUTH_OAUTH_FAILED');
}

function exactPath(url, path) {
  return url === ORIGIN + path || url.startsWith(ORIGIN + path + '?');
}

function header(headers, name) {
  const key = Object.keys(headers).find((value) => value.toLowerCase() === name);
  return key === undefined ? undefined : headers[key];
}

function nonceValue(cookie) {
  return typeof cookie === 'string'
    ? cookie.match(/(?:^|;\s*)iam-oauth-browser=([^;\s]+)/)?.[1] : undefined;
}

function cookieAttributes(cookie) {
  return typeof cookie === 'string' && cookie.startsWith('iam-oauth-browser=')
    && Boolean(nonceValue(cookie)) && /;\s*HttpOnly(?:;|$)/i.test(cookie)
    && /;\s*SameSite=Lax(?:;|$)/i.test(cookie) && /;\s*Max-Age=600(?:;|$)/i.test(cookie)
    && /;\s*Path=\/iam(?:;|$)/i.test(cookie) && !/;\s*(?:Secure(?:;|$)|Domain=)/i.test(cookie);
}

function authorization(url, redirect) {
  // Fixed origin/path, no rewriting or authority fallback; retain the URL intact.
  if (typeof url !== 'string' || !url.startsWith(PROVIDER + '?') || url.includes('#')) return false;
  const params = Object.create(null);
  for (const field of url.slice(PROVIDER.length + 1).split('&')) {
    const equal = field.indexOf('=');
    if (equal < 1) return false;
    const key = decodeURIComponent(field.slice(0, equal).replace(/\+/g, ' '));
    if (Object.prototype.hasOwnProperty.call(params, key)) return false;
    params[key] = decodeURIComponent(field.slice(equal + 1).replace(/\+/g, ' '));
  }
  return params.response_type === 'code' && Boolean(params.client_id) && Boolean(params.state)
    && params.redirect_uri === ORIGIN + redirect && params.code_challenge_method === 'S256'
    && typeof params.code_challenge === 'string' && /^[A-Za-z0-9_-]{43}$/.test(params.code_challenge);
}

export default async function () {
  started.add(1);
  let context, page;
  try {
    if (__ENV.K6_ORIGIN !== ORIGIN || !/^[a-f0-9]{32}$/.test(__ENV.K6_RUN_ID)
        || !__ENV.K6_SUMMARY_PATH) throw new Error('AUTH_INPUT_INVALID');
    const runId = __ENV.K6_RUN_ID;
    const expectedEmail = `${runId}@example.invalid`;
    context = await browser.newContext();
    expect('browser-version', __ENV.K6_EXPECTED_BROWSER_VERSION === '156.0.8078.73'
      && browser.version() === __ENV.K6_EXPECTED_BROWSER_VERSION);
    page = await context.newPage();
    const requests = [], responses = [], providerRequests = [], pending = [], nonces = [];
    let observationFailed = false;
    page.on('request', (request) => {
      try {
        const url = request.url();
        const path = PATHS.find((candidate) => exactPath(url, candidate));
        const provider = url === PROVIDER || url.startsWith(PROVIDER + '?');
        if (!path && !provider) return;
        const record = { url, path, method: request.method(), headers: null };
        (provider ? providerRequests : requests).push(record);
        pending.push(request.allHeaders().then((headers) => { record.headers = headers; })
          .catch(() => { observationFailed = true; }));
      } catch { observationFailed = true; }
    });
    page.on('response', (response) => {
      try {
        const path = PATHS.find((candidate) => exactPath(response.url(), candidate));
        if (!path) return;
        const record = { path, status: response.status(), headers: null };
        responses.push(record);
        pending.push(response.allHeaders().then((headers) => { record.headers = headers; })
          .catch(() => { observationFailed = true; }));
      } catch { observationFailed = true; }
    });
    async function observeNonce(name) {
      const cookies = (await context.cookies(ORIGIN + CALLBACK))
        .filter((cookie) => cookie.name === 'iam-oauth-browser');
      const nonce = cookies[0];
      expect(name, cookies.length === 1 && Boolean(nonce.value) && nonce.domain === '127.0.0.1'
        && nonce.path === '/iam' && nonce.httpOnly && !nonce.secure && nonce.sameSite === 'Lax');
      nonces.push(nonce.value);
    }
    const first = await page.goto(ORIGIN + LOGIN, { timeout: 15000 });
    expect('registration-callback', first && first.status() === 202 && exactPath(first.url(), CALLBACK));
    const registration = await first.json();
    expect('registration-required', registration.operation === 'registration_required'
      && typeof registration.registration_token === 'string' && registration.registration_token.length > 0);
    await observeNonce('registration-nonce');
    const registered = await page.evaluate(async ({ registration_token, username }) => {
      const response = await fetch('/iam/api/auth/complete-registration', {
        method: 'POST', credentials: 'same-origin', redirect: 'error',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ registration_token, username }),
      });
      return response.status;
    }, { registration_token: registration.registration_token, username: runId });
    expect('registration-completed', registered === 200 || registered === 201);

    // A fresh navigation is required: 202 preparation is never accepted as login.
    const login = await page.goto(ORIGIN + LOGIN, { timeout: 15000 });
    expect('login-callback', login && login.status() === 200 && exactPath(login.url(), CALLBACK));
    const result = await login.json();
    expect('login-identity', result.operation === 'login' && typeof result.access_token === 'string'
      && result.access_token.length > 0 && result.user?.email === expectedEmail && Boolean(result.user?.id));
    // Observation, not local trust verification: this exact token must pass Envoy at START.
    const claims = JSON.parse(encoding.b64decode(result.access_token.split('.')[1], 'rawurl', 's'));
    expect('login-authority', claims.iss === 'http://127.0.0.1:8080/iam' && claims.aud === 'aiforall');
    await observeNonce('login-nonce');
    const start = await page.evaluate(async (token) => {
      const response = await fetch('/iam/api/auth/gitlab/relink-start', {
        method: 'GET', credentials: 'same-origin', redirect: 'error',
        headers: { Authorization: 'Bearer ' + token },
      });
      return { status: response.status, body: await response.json() };
    }, result.access_token);
    expect('relink-start', start.status === 200 && authorization(start.body.auth_url, RELINK));
    const relink = await page.goto(start.body.auth_url, { timeout: 15000 });
    expect('relink-callback', relink && relink.status() === 200 && exactPath(relink.url(), RELINK));
    const linked = await relink.json();
    expect('relink-identity', linked.user?.id === result.user.id && linked.user?.email === expectedEmail
      && Array.isArray(linked.emails));
    await observeNonce('relink-nonce');
    for (const [path, name] of NEGATIVES) {
      const status = await page.evaluate(async (path) => {
        const response = await fetch(path, { method: 'GET', credentials: 'same-origin', redirect: 'error' });
        return response.status;
      }, path);
      expect(name, status === 403);
    }
    await Promise.all(pending);
    expect('oauth-observations', !observationFailed
      && [...requests, ...responses, ...providerRequests].every((record) => record.headers !== null));
    const req = (path) => requests.filter((record) => record.path === path);
    const res = (path) => responses.filter((record) => record.path === path);
    const cardinalities = [[LOGIN, 2], [CALLBACK, 2], [REGISTER, 1], [START, 2], [RELINK, 1],
      [NEGATIVES[0][0], 1], [NEGATIVES[2][0], 1]];
    expect('oauth-iam-cardinality', requests.length === 10 && responses.length === 10
      && cardinalities.every(([path, count]) => req(path).length === count && res(path).length === count));
    expect('login-redirects', req(LOGIN).every((r) => r.method === 'GET' && header(r.headers, 'authorization') === undefined)
      && res(LOGIN).every((r, index) => r.status === 303
        && authorization(header(r.headers, 'location'), CALLBACK)
        && header(r.headers, 'location') === providerRequests[index]?.url));
    const cookieResponses = [...res(LOGIN), ...res(START).filter((r) => r.status === 200)];
    expect('oauth-cookie-attributes', cookieAttributes(header(res(LOGIN)[0].headers, 'set-cookie'))
      && cookieResponses.every((r) => header(r.headers, 'set-cookie') === undefined
        || cookieAttributes(header(r.headers, 'set-cookie'))));
    const callbacks = [...req(CALLBACK), ...req(RELINK)];
    expect('oauth-callback-headers', callbacks.length === 3 && callbacks.every((r, index) => r.method === 'GET'
      && header(r.headers, 'authorization') === undefined && nonceValue(header(r.headers, 'cookie')) === nonces[index])
      && [...res(CALLBACK), ...res(RELINK)].every((r) => header(r.headers, 'set-cookie') === undefined));
    expect('registration-request-headers', req(REGISTER)[0].method === 'POST'
      && header(req(REGISTER)[0].headers, 'authorization') === undefined);
    const authenticated = req(START).filter((r) => header(r.headers, 'authorization') !== undefined);
    expect('relink-start-authorization', authenticated.length === 1 && authenticated[0].method === 'GET'
      && header(authenticated[0].headers, 'authorization') === 'Bearer ' + result.access_token);
    expect('provider-authorizations', providerRequests.length === 3
      && providerRequests.every((r, index) => r.method === 'GET' && authorization(r.url, index < 2 ? CALLBACK : RELINK))
      && providerRequests[2].url === start.body.auth_url);
    expect('provider-no-credentials', providerRequests.every((r) => header(r.headers, 'authorization') === undefined
      && header(r.headers, 'cookie') === undefined));
    expect('negative-headers', NEGATIVES.every(([path]) => {
      const negative = req(path).filter((r) => header(r.headers, 'authorization') === undefined);
      const denied = res(path).filter((r) => r.status === 403);
      return negative.length === 1 && negative[0].method === 'GET' && denied.length === 1
        && header(denied[0].headers, 'set-cookie') === undefined;
    }));
  } catch {
    // Native errors may contain OAuth URLs/code/state or bodies; never propagate.
    throw new Error('AUTH_OAUTH_FAILED');
  } finally {
    try {
      try { if (page) await page.close(); } finally { if (context) await context.close(); }
    } catch { throw new Error('AUTH_OAUTH_CLEANUP_FAILED'); }
  }
  completed.add(1);
}

export function handleSummary(data) { return project(data, 'Chromium', 'auth-oauth-browser'); }

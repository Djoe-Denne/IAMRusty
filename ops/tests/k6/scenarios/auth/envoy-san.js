import http from 'k6/http';
import { check } from 'k6';
import { started, completed, project } from '../../lib/summary.js';

// One HTTP witness per parent-owned phase; never mutate certificates or access
// Envoy Admin here. 503 alone is NOT proof of SAN rejection. Parent must supply
// same-incarnation fail_verify_san delta, same-CA identity, ACK and restoration.
// Closed checks, each passes=1/fails=0: san-status, san-no-redirect.
const PHASE = __ENV.K6_AUTH_SAN_PHASE;
if (!['good', 'bad', 'restored'].includes(PHASE)) throw new Error('AUTH_SAN_PHASE_INVALID');
const CASE = `auth-envoy-san-${PHASE}`;
const LOGIN = 'http://127.0.0.1:18080/iam/api/auth/gitlab/login';

export const options = {
  scenarios: {
    [CASE]: {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '15s', gracefulStop: '2s',
    },
  },
  thresholds: {
    checks: ['rate==1'], case_started: ['count==1'], case_completed: ['count==1'], http_reqs: ['count==1'],
  },
  systemTags: ['name', 'check', 'scenario'],
  maxRedirects: 0, discardResponseBodies: true,
};

export default function () {
  started.add(1);
  if (__ENV.K6_ORIGIN !== 'http://127.0.0.1:18080' || !/^[a-f0-9]{32}$/.test(__ENV.K6_RUN_ID)
      || !__ENV.K6_SUMMARY_PATH) throw new Error('AUTH_INPUT_INVALID');
  const response = http.get(LOGIN, { redirects: 0, timeout: '5s', tags: { name: CASE } });
  check(response, {
    'san-status': (r) => r.status === (PHASE === 'bad' ? 503 : 303),
    'san-no-redirect': (r) => r.url === LOGIN,
  });
  completed.add(1);
}

export function handleSummary(data) { return project(data, 'HTTP', CASE); }

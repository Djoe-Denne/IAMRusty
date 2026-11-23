import http from 'k6/http';
import { scenarioContract } from '../../lib/case-contract.js';
import { campaignTarget } from '../../lib/campaign-target.js';

// One HTTP witness per parent-owned phase; never mutate certificates or access
// Envoy Admin here. 503 alone is NOT proof of SAN rejection. Parent must supply
// same-incarnation fail_verify_san delta, same-CA identity, ACK and restoration.
// Closed checks, each passes=1/fails=0: san-status, san-no-redirect.
const PHASE = __ENV.K6_AUTH_SAN_PHASE;
if (!['good', 'bad', 'restored'].includes(PHASE)) throw new Error('AUTH_SAN_PHASE_INVALID');
const CASE = `auth-envoy-san-${PHASE}`;
const contract = scenarioContract(CASE);
const target = campaignTarget();
const LOGIN = target.origin + '/iam/api/auth/gitlab/login';

export const options = {
  scenarios: {
    [CASE]: {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '15s', gracefulStop: '2s',
    },
  },
  thresholds: contract.thresholds,
  systemTags: ['name', 'check', 'scenario', 'group'],
  maxRedirects: 0, discardResponseBodies: true,
};

export default function () {
  contract.start();
  if (__ENV.K6_ORIGIN !== target.origin || !/^[a-f0-9]{32}$/.test(__ENV.K6_RUN_ID)
      || !__ENV.K6_SUMMARY_PATH) throw new Error('AUTH_INPUT_INVALID');
  const response = http.get(LOGIN, { redirects: 0, timeout: '5s', tags: { name: CASE } });
  contract.check(response, {
    'san-status': (r) => r.status === (PHASE === 'bad' ? 503 : 303),
    'san-no-redirect': (r) => r.url === LOGIN,
  });
  contract.finish();
}

export function handleSummary(data) { return contract.summary(data); }

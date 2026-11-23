import http from 'k6/http';
import { sleep } from 'k6';
import { baseline } from '../../lib/summary.js';
import { scenarioContract } from '../../lib/case-contract.js';
import { campaignTarget } from '../../lib/campaign-target.js';

const target = campaignTarget();
const contract = scenarioContract('iam-health');

export const options = {
  scenarios: {
    'iam-health': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '15s', gracefulStop: '2s',
    },
  },
  thresholds: contract.thresholds,
  systemTags: ['name', 'method', 'status', 'check', 'scenario', 'group'],
  maxRedirects: 0, discardResponseBodies: false,
};

export default function () {
  if (__ENV.K6_ORIGIN !== target.origin || !/^[a-f0-9]{32}$/.test(__ENV.K6_RUN_ID)) {
    throw new Error('INPUT_INVALID');
  }
  if (__ENV.K6_INTERNAL_NEGATIVE === 'absent-case') return;
  if (__ENV.K6_INTERNAL_NEGATIVE === 'interrupt') { sleep(20); return; }
  contract.start();
  const response = http.get(`${__ENV.K6_ORIGIN}/iam/health`, {
    redirects: 0, timeout: '5s', tags: { name: 'iam-health' },
    responseType: 'text',
  });
  if (__ENV.K6_INTERNAL_NEGATIVE === 'false-check') baseline.add(response.status === 200 && response.body === 'OK' ? 1 : 0);
  contract.check(response, {
    'iam-status': (r) => r.status === 200,
    'iam-body': (r) => __ENV.K6_INTERNAL_NEGATIVE === 'false-check' ? r.body === 'INTENTIONALLY_FALSE' : r.body === 'OK',
  });
  contract.finish();
}

export function handleSummary(data) { return contract.summary(data); }

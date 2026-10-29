import http from 'k6/http';
import { check, sleep } from 'k6';
import { started, completed, baseline, project } from '../../lib/summary.js';

export const options = {
  scenarios: {
    'iam-health': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '15s', gracefulStop: '2s',
    },
  },
  thresholds: { checks: ['rate==1'], case_started: ['count==1'], case_completed: ['count==1'] },
  systemTags: ['name', 'method', 'status', 'check', 'scenario'],
  maxRedirects: 0, discardResponseBodies: false,
};

export default function () {
  if (__ENV.K6_ORIGIN !== 'http://127.0.0.1:18080' || !/^[a-f0-9]{32}$/.test(__ENV.K6_RUN_ID)) {
    throw new Error('INPUT_INVALID');
  }
  if (__ENV.K6_INTERNAL_NEGATIVE === 'absent-case') return;
  if (__ENV.K6_INTERNAL_NEGATIVE === 'interrupt') { sleep(20); return; }
  started.add(1);
  const response = http.get(`${__ENV.K6_ORIGIN}/iam/health`, {
    redirects: 0, timeout: '5s', tags: { name: 'iam-health' },
    responseType: 'text',
  });
  if (__ENV.K6_INTERNAL_NEGATIVE === 'false-check') baseline.add(response.status === 200 && response.body === 'OK' ? 1 : 0);
  check(response, {
    'iam-status': (r) => r.status === 200,
    'iam-body': (r) => __ENV.K6_INTERNAL_NEGATIVE === 'false-check' ? r.body === 'INTENTIONALLY_FALSE' : r.body === 'OK',
  });
  completed.add(1);
}

export function handleSummary(data) { return project(data, 'HTTP', 'iam-health'); }

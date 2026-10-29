// No remote imports and no export of raw metrics, URLs, tags or exceptions.
import { Counter } from 'k6/metrics';

export const started = new Counter('case_started');
export const completed = new Counter('case_completed');
export const baseline = new Counter('selftest_baseline');

export function project(data, phase, caseId) {
  const count = (name) => data.metrics[name]?.values?.count ?? null;
  const checks = {};
  function visit(group) {
    for (const c of group.checks ?? []) {
      if (Object.prototype.hasOwnProperty.call(checks, c.name)) {
        throw new Error('SUMMARY_DUPLICATE_CHECK');
      }
      checks[c.name] = { passes: c.passes, fails: c.fails };
    }
    for (const child of group.groups ?? []) visit(child);
  }
  visit(data.root_group);
  const value = {
    version: 1, run_id: __ENV.K6_RUN_ID, phase,
    cases: [caseId], expected: 1,
    started: count('case_started'), completed: count('case_completed'),
    iterations: count('iterations'),
    // A browser-only scenario does not instantiate k6/http.
    requests: phase === 'HTTP' ? count('http_reqs') : 0,
    checks,
  };
  if (__ENV.K6_INTERNAL_NEGATIVE === 'false-check') value.negative_baseline = count('selftest_baseline');
  if (__ENV.K6_INTERNAL_NEGATIVE === 'missing-summary') return {};
  return { [__ENV.K6_SUMMARY_PATH]: JSON.stringify(value) };
}

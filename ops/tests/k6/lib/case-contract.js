import { check as k6Check } from 'k6';
import execution from 'k6/execution';
import { Counter } from 'k6/metrics';
import { completed, project, started } from './summary.js';

const catalogue = JSON.parse(open('../scenarios/catalog.json'));
const executions = new Counter('case_check_executions');
const identifier = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function scenarioContract(id) {
  if (catalogue.version !== 1 || !Array.isArray(catalogue.cases)) {
    throw new Error('CASE_CATALOG_INVALID');
  }
  if (__ENV.K6_CASE_ID !== undefined && __ENV.K6_CASE_ID !== id) {
    throw new Error('CASE_IDENTITY_MISMATCH');
  }
  const matches = catalogue.cases.filter((entry) => entry.id === id);
  if (matches.length !== 1) throw new Error('CASE_NOT_REGISTERED');
  const { checks, phase, requests } = matches[0];
  const expected = Object.keys(checks);
  if (!expected.length || expected.some((name) => name.length > 64 || !identifier.test(name)
      || !['functional', 'cleanup'].includes(checks[name]))) {
    throw new Error('CASE_CHECKS_INVALID');
  }
  const names = new Set(expected);
  const thresholds = {
    checks: ['rate==1'],
    case_started: ['count==1'],
    case_completed: ['count==1'],
    iterations: ['count==1'],
    http_reqs: [`count==${requests}`],
  };
  for (const name of names) thresholds[`case_check_executions{assertion:${name}}`] = ['count==1'];

  return {
    thresholds,
    start() {
      // Initialize every submetric, even when the assertion never runs.
      for (const name of names) executions.add(0, { assertion: name });
      started.add(1);
    },
    check(value, predicates) {
      const observed = Object.keys(predicates);
      if (!observed.length || observed.some((name) => !names.has(name))) {
        execution.test.abort('CASE_CHECK_NOT_DECLARED');
      }
      const passed = k6Check(value, predicates);
      for (const name of observed) executions.add(1, { assertion: name });
      return passed;
    },
    finish() { completed.add(1); },
    summary(data) { return project(data, phase, id, requests); },
  };
}

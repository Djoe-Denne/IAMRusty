// Diagnostic only: no navigation, app traffic or E2E qualification.
import { browser } from 'k6/browser';
import { Counter } from 'k6/metrics';

const stages = {
  started: new Counter('sandbox_probe_started'),
  context: new Counter('sandbox_probe_context'),
  version: new Counter('sandbox_probe_version'),
  page: new Counter('sandbox_probe_page'),
  evaluation: new Counter('sandbox_probe_evaluation'),
};

export const options = {
  scenarios: {
    'sandbox-observation': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '12s', gracefulStop: '2s', options: { browser: { type: 'chromium' } },
    },
  },
};

export default async function () {
  stages.started.add(1);
  const context = await browser.newContext();
  stages.context.add(1);
  try {
    if (browser.version() !== __ENV.K6_EXPECTED_BROWSER_VERSION) throw new Error('PINNED_BROWSER_VERSION_REQUIRED');
    stages.version.add(1);
    const page = await context.newPage();
    stages.page.add(1);
    try {
      // Keep the live renderer observable across the supervisor's sampling
      // cycles; a completed evaluation alone is never sandbox evidence.
      if (await page.evaluate(async () => {
        await new Promise((resolve) => setTimeout(resolve, 1500));
        return 2 + 2;
      }) !== 4) throw new Error('RENDERER_EVALUATION_FAILED');
      stages.evaluation.add(1);
    } finally { await page.close(); }
  } finally { await context.close(); }
}

export function handleSummary(data) {
  const result = {};
  for (const stage of Object.keys(stages)) {
    result[stage] = data.metrics[`sandbox_probe_${stage}`]?.values?.count ?? 0;
  }
  return { [__ENV.K6_SUMMARY_PATH]: JSON.stringify(result) };
}

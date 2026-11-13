// Diagnostic only: no navigation, app traffic or E2E qualification.
import { browser } from 'k6/browser';

export const options = {
  scenarios: {
    'sandbox-observation': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '12s', gracefulStop: '2s', options: { browser: { type: 'chromium' } },
    },
  },
};

export default async function () {
  const context = await browser.newContext();
  try {
    if (browser.version() !== __ENV.K6_EXPECTED_BROWSER_VERSION) throw new Error('PINNED_BROWSER_VERSION_REQUIRED');
    const page = await context.newPage();
    try {
      if (await page.evaluate(() => 2 + 2) !== 4) throw new Error('RENDERER_EVALUATION_FAILED');
    } finally { await page.close(); }
  } finally { await context.close(); }
}

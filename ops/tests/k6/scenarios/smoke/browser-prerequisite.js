import { browser } from 'k6/browser';
import { check } from 'k6';
import { started, completed, project } from '../../lib/summary.js';

export const options = {
  scenarios: {
    'browser-prerequisite': {
      executor: 'per-vu-iterations', vus: 1, iterations: 1,
      maxDuration: '20s', gracefulStop: '3s', options: { browser: { type: 'chromium' } },
    },
  },
  thresholds: { checks: ['rate==1'], case_started: ['count==1'], case_completed: ['count==1'] },
  systemTags: ['name', 'check', 'scenario'],
};

export default async function () {
  started.add(1);
  const context = await browser.newContext();
  let page;
  try {
    page = await context.newPage();
    await page.goto('about:blank', { timeout: 5000 });
    check(await page.evaluate(() => 6 * 7), { 'browser-evaluation': (value) => value === 42 });
    check(page.url(), { 'browser-blank': (value) => value === 'about:blank' });
    // Bounded observation window for the parent's /proc sandbox witness.
    await page.waitForTimeout(750);
  } finally {
    try { if (page) await page.close(); } finally { await context.close(); }
  }
  completed.add(1);
}

export function handleSummary(data) { return project(data, 'Chromium', 'browser-prerequisite'); }

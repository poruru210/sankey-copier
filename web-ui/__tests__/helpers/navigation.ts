import type { Page } from '@playwright/test';

const DEFAULT_BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:8080';
const NORMALIZED_BASE_URL = DEFAULT_BASE_URL.endsWith('/')
  ? DEFAULT_BASE_URL.slice(0, -1)
  : DEFAULT_BASE_URL;

async function seedSiteStorage(page: Page, siteUrl: string) {
  const seededSite = {
    id: 'playwright-local-site',
    name: 'Playwright Local',
    siteUrl,
  };

  await page.addInitScript(({ siteData }) => {
    window.localStorage.setItem('sankey-copier-sites', JSON.stringify([siteData]));
    window.localStorage.setItem('sankey-copier-selected-site-id', siteData.id);
  }, { siteData: seededSite });

  return seededSite;
}

/**
 * Navigate to an absolute or relative path while respecting the configured base URL.
 */
export async function gotoApp(page: Page, path = '/', options?: { siteUrl?: string }) {
  const baseUrl = (options?.siteUrl ?? NORMALIZED_BASE_URL).replace(/\/$/, '');
  const seededSite = await seedSiteStorage(page, baseUrl);

  const target = path.startsWith('http')
    ? path
    : `${baseUrl}${path.startsWith('/') ? path : `/${path}`}`;
  await page.goto(target);

  await page.evaluate(({ siteData }) => {
    const serializedSites = JSON.stringify([siteData]);
    window.localStorage.setItem('sankey-copier-sites', serializedSites);
    window.localStorage.setItem('sankey-copier-selected-site-id', siteData.id);
    window.dispatchEvent(new StorageEvent('storage', { key: 'sankey-copier-sites', newValue: serializedSites }));
    window.dispatchEvent(new StorageEvent('storage', { key: 'sankey-copier-selected-site-id', newValue: siteData.id }));
  }, { siteData: seededSite });

  if (process.env.PLAYWRIGHT_LOG_API === '1') {
    const storageSnapshot = await page.evaluate(() => ({
      sites: window.localStorage.getItem('sankey-copier-sites'),
      selected: window.localStorage.getItem('sankey-copier-selected-site-id'),
    }));
    console.log('[Navigation] storage snapshot for test', storageSnapshot);
  }
}

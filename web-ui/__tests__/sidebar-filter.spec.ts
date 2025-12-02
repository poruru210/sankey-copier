import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';
import { gotoApp } from './helpers/navigation';
import { setupDefaultApiMocks, installBasicWebSocketMock } from './helpers/api';

const SELECTORS = {
  trigger: '[data-testid="master-filter-trigger"]',
  menu: '[data-testid="master-filter-menu"]',
  indicator: '[data-testid="master-filter-indicator"]',
  count: '[data-testid="master-filter-count"]',
  optionAll: '[data-testid="master-filter-option-all"]',
  optionFxPro: '[data-testid="master-filter-option-FxPro_12345001"]',
  optionOanda: '[data-testid="master-filter-option-OANDA_67890002"]',
  optionXm: '[data-testid="master-filter-option-XM_11111003"]',
};

async function openFilterMenu(page: Page) {
  const trigger = page.locator(SELECTORS.trigger);
  const menu = page.locator(SELECTORS.menu);

  await trigger.click();
  try {
    await menu.waitFor({ state: 'visible', timeout: 2000 });
  } catch {
    // Retry once more in case the dropdown was mid-transition
    await trigger.click({ force: true });
    await menu.waitFor({ state: 'visible' });
  }
}

async function selectMaster(page: Page, masterId: string) {
  await openFilterMenu(page);
  const option = page.locator(`[data-testid="master-filter-option-${masterId}"]`);
  await option.click();
  await page.locator(SELECTORS.menu).waitFor({ state: 'hidden' });
}

test.describe('Master account filter', () => {
  test.beforeEach(async ({ page }) => {
    await setupDefaultApiMocks(page);
    await installBasicWebSocketMock(page);
    await gotoApp(page);
    await page.waitForSelector(SELECTORS.trigger, { timeout: 10_000 });
  });

  test('should show "All Accounts" summary with total links', async ({ page }) => {
    const trigger = page.locator(SELECTORS.trigger);
    await expect(trigger).toBeVisible();
    await expect(trigger).toContainText('All Accounts');
    await expect(trigger.locator(SELECTORS.count)).toHaveText('3');
  });

  test('should list every master with connection counts', async ({ page }) => {
    await openFilterMenu(page);

    await expect(page.locator(SELECTORS.optionAll)).toContainText('All Accounts');
    await expect(page.locator(SELECTORS.optionFxPro)).toContainText('FxPro');
    await expect(page.locator(SELECTORS.optionFxPro)).toContainText('#12345001');
    await expect(page.locator(SELECTORS.optionFxPro)).toContainText('2 links');

    await expect(page.locator(SELECTORS.optionOanda)).toContainText('OANDA');
    await expect(page.locator(SELECTORS.optionOanda)).toContainText('#67890002');
    await expect(page.locator(SELECTORS.optionOanda)).toContainText('1 link');

    await expect(page.locator(SELECTORS.optionXm)).toContainText('XM');
    await expect(page.locator(SELECTORS.optionXm)).toContainText('#11111003');
    await expect(page.locator(SELECTORS.optionXm)).toContainText('0 links');
  });

  test('should show online/offline state badges inside the dropdown', async ({ page }) => {
    await openFilterMenu(page);
    await expect(page.locator(SELECTORS.optionFxPro)).toContainText('Online');
    await expect(page.locator(SELECTORS.optionOanda)).toContainText('Offline');
  });

  test('should filter the graph when a master is selected', async ({ page }) => {
    await selectMaster(page, 'FxPro_12345001');

    const indicator = page.locator(SELECTORS.indicator);
    await expect(indicator).toBeVisible();
    await expect(indicator).toContainText('Viewing');
    await expect(indicator).toContainText('Trading Account A');

    const trigger = page.locator(SELECTORS.trigger);
    await expect(trigger).toContainText('FxPro');
    await expect(trigger.locator(SELECTORS.count)).toHaveText('2');
  });

  test('should update summary count when switching masters', async ({ page }) => {
    await selectMaster(page, 'OANDA_67890002');
    await expect(page.locator(SELECTORS.count)).toHaveText('1');

    await selectMaster(page, 'XM_11111003');
    await expect(page.locator(SELECTORS.count)).toHaveText('0');
  });

  test('should clear the filter via the indicator button', async ({ page }) => {
    await selectMaster(page, 'FxPro_12345001');
    await expect(page.locator(SELECTORS.indicator)).toBeVisible();

    await page.getByRole('button', { name: /clear filter/i }).click();

    await expect(page.locator(SELECTORS.indicator)).toHaveCount(0);
    const trigger = page.locator(SELECTORS.trigger);
    await expect(trigger).toContainText('All Accounts');
    await expect(trigger.locator(SELECTORS.count)).toHaveText('3');
  });
});

test.describe('Master filter on mobile', () => {
  test.use({ viewport: { width: 375, height: 667 } });

  test.beforeEach(async ({ page }) => {
    await setupDefaultApiMocks(page);
    await installBasicWebSocketMock(page);
    await gotoApp(page);
    await page.waitForSelector(SELECTORS.trigger, { timeout: 10_000 });
  });

  test('should open and close the dropdown via tap', async ({ page }) => {
    await openFilterMenu(page);
    await expect(page.locator(SELECTORS.menu)).toBeVisible();

    // Close via keyboard escape to mimic tapping outside
    await page.keyboard.press('Escape');
    await expect(page.locator(SELECTORS.menu)).toHaveCount(0);
  });

  test('should apply a master filter on mobile', async ({ page }) => {
    await selectMaster(page, 'OANDA_67890002');
    await expect(page.locator(SELECTORS.indicator)).toContainText('Trading Account B');
  });
});

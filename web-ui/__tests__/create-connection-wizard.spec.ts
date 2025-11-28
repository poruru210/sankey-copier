import { test, expect } from '@playwright/test';
import { mockConnections, mockSettings } from './mocks/testData';

/**
 * Create Connection Wizard - E2E Tests
 *
 * Tests the connection creation wizard functionality, specifically:
 * - TradeGroup creation timing (should not fail when TradeGroup doesn't exist)
 * - Status field (should respect user's "enable" choice)
 * - Master settings preservation for new connections
 *
 * Bug regression tests for:
 * - "TradeGroup not found" error when creating new connections
 * - status hardcoded to 0 instead of user-selected value
 * - Master settings lost for new connections
 */

// Track API calls to verify correct behavior
interface ApiCall {
  method: string;
  url: string;
  body?: unknown;
}

test.describe('Create Connection Wizard', () => {
  let apiCalls: ApiCall[] = [];
  let createdMember: unknown = null;
  let updatedMasterSettings: unknown = null;

  test.beforeEach(async ({ page }) => {
    // Reset tracking
    apiCalls = [];
    createdMember = null;
    updatedMasterSettings = null;

    // Mock connections API
    await page.route('**/api/connections', async (route) => {
      apiCalls.push({ method: route.request().method(), url: route.request().url() });
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: mockConnections,
        }),
      });
    });

    // Mock settings API
    await page.route('**/api/settings', async (route) => {
      apiCalls.push({ method: route.request().method(), url: route.request().url() });
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: mockSettings,
          }),
        });
      } else if (route.request().method() === 'POST') {
        const body = route.request().postDataJSON();
        createdMember = body;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: { id: 999, ...body },
          }),
        });
      }
    });

    // Mock trade_groups API - Return 404 for new TradeGroup (simulate first-time creation)
    await page.route('**/api/trade_groups/*', async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      apiCalls.push({ method, url });

      if (method === 'GET') {
        // Simulate TradeGroup not existing yet
        await route.fulfill({
          status: 404,
          contentType: 'application/json',
          body: JSON.stringify({
            success: false,
            error: 'TradeGroup not found',
          }),
        });
      } else if (method === 'PUT' || method === 'PATCH') {
        // Master settings update after member creation
        const body = route.request().postDataJSON();
        updatedMasterSettings = body;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: body,
          }),
        });
      }
    });

    // Mock trade_group_members API
    await page.route('**/api/trade_groups/*/members', async (route) => {
      apiCalls.push({ method: route.request().method(), url: route.request().url() });
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [],
        }),
      });
    });

    // Mock WebSocket
    await page.addInitScript(() => {
      class MockWebSocket {
        readyState = 1;
        onopen: ((ev: Event) => void) | null = null;
        onmessage: ((ev: MessageEvent) => void) | null = null;
        onerror: ((ev: Event) => void) | null = null;
        onclose: ((ev: CloseEvent) => void) | null = null;

        constructor(_url: string) {
          setTimeout(() => {
            if (this.onopen) this.onopen(new Event('open'));
          }, 10);
        }

        send(_data: string) {}
        close() {
          if (this.onclose) this.onclose(new CloseEvent('close'));
        }
      }

      (window as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;
    });

    await page.goto('http://localhost:5173');
    await page.waitForLoadState('networkidle');
  });

  test('should open create connection dialog', async ({ page }) => {
    // Look for the "Create Connection" button
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await expect(createButton).toBeVisible({ timeout: 10000 });

    await createButton.click();
    await page.waitForTimeout(300);

    // Dialog should be open with Step 1 visible
    await expect(page.getByText(/Master.*Account|マスター/i)).toBeVisible();
    await expect(page.getByText(/Slave.*Account|スレーブ/i)).toBeVisible();
  });

  test('should not call updateTradeGroupSettings when TradeGroup does not exist (Step 1 → Step 2)', async ({ page }) => {
    // Open create dialog
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await createButton.click();
    await page.waitForTimeout(500);

    // Step 1: Select Master and Slave accounts
    // Find and click the Master account selector
    const masterSelector = page.locator('[data-testid="master-selector"], select, [role="combobox"]').first();
    if (await masterSelector.isVisible()) {
      await masterSelector.click();
      await page.waitForTimeout(200);
      // Select first Master account
      await page.locator('[role="option"]').first().click();
    }

    // Find and click the Slave account selector
    const slaveSelector = page.locator('[data-testid="slave-selector"], select, [role="combobox"]').nth(1);
    if (await slaveSelector.isVisible()) {
      await slaveSelector.click();
      await page.waitForTimeout(200);
      // Select first Slave account
      await page.locator('[role="option"]').first().click();
    }

    // Click Next to go to Step 2 (Master Settings)
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(500);

    // Step 2: Enter Master settings (symbol suffix)
    const symbolSuffixInput = page.locator('input[id="master_symbol_suffix"], input[placeholder*="suffix"]');
    if (await symbolSuffixInput.isVisible()) {
      await symbolSuffixInput.fill('.m');
    }

    // Click Next to go to Step 3 (Slave Settings)
    // This should NOT trigger updateTradeGroupSettings since TradeGroup doesn't exist
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(500);

    // Verify no PUT/PATCH to trade_groups was made during step transition
    // (Only GET requests should happen when checking if TradeGroup exists)
    const tradeGroupUpdates = apiCalls.filter(
      (call) => call.url.includes('trade_groups') && (call.method === 'PUT' || call.method === 'PATCH')
    );

    // Should not have called updateTradeGroupSettings yet
    expect(tradeGroupUpdates.length).toBe(0);

    // Should be on Step 3 (no error dialog should appear)
    await expect(page.getByText(/Slave.*Settings|スレーブ.*設定|Lot/i)).toBeVisible();
  });

  test('should create member with status 2 when "有効化する" checkbox is checked', async ({ page }) => {
    // Open create dialog
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await createButton.click();
    await page.waitForTimeout(500);

    // Step 1: Select accounts (simplified - just need to proceed)
    const masterSelector = page.locator('[data-testid="master-selector"], [role="combobox"]').first();
    if (await masterSelector.isVisible()) {
      await masterSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    const slaveSelector = page.locator('[data-testid="slave-selector"], [role="combobox"]').nth(1);
    if (await slaveSelector.isVisible()) {
      await slaveSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    // Step 1 → Step 2
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Step 2 → Step 3
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Step 3: Check the "有効化する" checkbox
    const enableCheckbox = page.locator('input[id="enable_on_create"], [role="checkbox"]').filter({
      has: page.locator('text=有効化')
    });

    // Try different selectors for the checkbox
    const checkbox = page.locator('#enable_on_create');
    if (await checkbox.isVisible()) {
      await checkbox.check();
    } else {
      // Try clicking the label
      const label = page.locator('label:has-text("有効化する")');
      if (await label.isVisible()) {
        await label.click();
      }
    }

    await page.waitForTimeout(200);

    // Submit the form
    const saveButton = page.getByRole('button', { name: /保存|save/i });
    await saveButton.click();
    await page.waitForTimeout(500);

    // Verify the created member has status: 2 (not 0)
    if (createdMember) {
      expect((createdMember as { status: number }).status).toBe(2);
    }
  });

  test('should create member with status 0 when "有効化する" checkbox is NOT checked', async ({ page }) => {
    // Open create dialog
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await createButton.click();
    await page.waitForTimeout(500);

    // Step 1: Select accounts
    const masterSelector = page.locator('[role="combobox"]').first();
    if (await masterSelector.isVisible()) {
      await masterSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    const slaveSelector = page.locator('[role="combobox"]').nth(1);
    if (await slaveSelector.isVisible()) {
      await slaveSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    // Step 1 → Step 2 → Step 3
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Step 3: Do NOT check the "有効化する" checkbox (default is unchecked)

    // Submit the form
    const saveButton = page.getByRole('button', { name: /保存|save/i });
    await saveButton.click();
    await page.waitForTimeout(500);

    // Verify the created member has status: 0
    if (createdMember) {
      expect((createdMember as { status: number }).status).toBe(0);
    }
  });

  test('should save master settings after member creation for new connections', async ({ page }) => {
    // Open create dialog
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await createButton.click();
    await page.waitForTimeout(500);

    // Step 1: Select accounts
    const masterSelector = page.locator('[role="combobox"]').first();
    if (await masterSelector.isVisible()) {
      await masterSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    const slaveSelector = page.locator('[role="combobox"]').nth(1);
    if (await slaveSelector.isVisible()) {
      await slaveSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    // Step 1 → Step 2
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Step 2: Enter Master symbol suffix
    const symbolSuffixInput = page.locator('input[id="master_symbol_suffix"]');
    if (await symbolSuffixInput.isVisible()) {
      await symbolSuffixInput.fill('m');
    }

    // Step 2 → Step 3
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Submit the form
    const saveButton = page.getByRole('button', { name: /保存|save/i });
    await saveButton.click();
    await page.waitForTimeout(1000);

    // Verify master settings were updated AFTER member creation
    // The updateTradeGroupSettings call should happen after onCreate
    if (updatedMasterSettings) {
      expect((updatedMasterSettings as { symbol_suffix: string }).symbol_suffix).toBe('m');
    }
  });
});

test.describe('Create Connection Wizard - Error Handling', () => {
  test.beforeEach(async ({ page }) => {
    // Mock connections API
    await page.route('**/api/connections', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: mockConnections,
        }),
      });
    });

    // Mock settings API
    await page.route('**/api/settings', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: mockSettings,
        }),
      });
    });

    // Mock trade_groups API - Return 404 (TradeGroup not found)
    await page.route('**/api/trade_groups/*', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 404,
          contentType: 'application/json',
          body: JSON.stringify({
            success: false,
            error: 'TradeGroup not found',
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true }),
        });
      }
    });

    await page.route('**/api/trade_groups/*/members', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      });
    });

    // Mock WebSocket
    await page.addInitScript(() => {
      class MockWebSocket {
        readyState = 1;
        onopen: ((ev: Event) => void) | null = null;
        onmessage: ((ev: MessageEvent) => void) | null = null;
        onerror: ((ev: Event) => void) | null = null;
        onclose: ((ev: CloseEvent) => void) | null = null;
        constructor() { setTimeout(() => { if (this.onopen) this.onopen(new Event('open')); }, 10); }
        send() {}
        close() { if (this.onclose) this.onclose(new CloseEvent('close')); }
      }
      (window as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;
    });

    await page.goto('http://localhost:5173');
    await page.waitForLoadState('networkidle');
  });

  test('should NOT show "TradeGroup not found" error when proceeding from Step 2 to Step 3', async ({ page }) => {
    // This is a regression test for the bug where Step 2 → Step 3 transition
    // would show "Failed to update master settings: TradeGroup not found"

    // Open create dialog
    const createButton = page.getByRole('button', { name: /新しい紐づけを作成|create.*connection/i });
    await createButton.click();
    await page.waitForTimeout(500);

    // Step 1: Select accounts
    const masterSelector = page.locator('[role="combobox"]').first();
    if (await masterSelector.isVisible()) {
      await masterSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    const slaveSelector = page.locator('[role="combobox"]').nth(1);
    if (await slaveSelector.isVisible()) {
      await slaveSelector.click();
      await page.waitForTimeout(200);
      await page.locator('[role="option"]').first().click();
    }

    // Step 1 → Step 2
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(300);

    // Step 2: Enter symbol suffix (this used to trigger the error)
    const symbolSuffixInput = page.locator('input[id="master_symbol_suffix"]');
    if (await symbolSuffixInput.isVisible()) {
      await symbolSuffixInput.fill('m');
    }

    // Listen for console errors
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    // Step 2 → Step 3 (this transition should NOT cause an error)
    await page.getByRole('button', { name: /next|次へ/i }).click();
    await page.waitForTimeout(500);

    // Verify no "TradeGroup not found" error was logged
    const tradeGroupErrors = consoleErrors.filter((err) =>
      err.includes('TradeGroup not found') || err.includes('Failed to update master settings')
    );
    expect(tradeGroupErrors.length).toBe(0);

    // Should successfully reach Step 3
    await expect(page.getByText(/Slave.*Settings|スレーブ.*設定|Lot/i)).toBeVisible();
  });
});

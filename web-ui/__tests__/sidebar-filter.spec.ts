import { test, expect } from '@playwright/test';
import { mockConnections, mockSettings } from './mocks/testData';

/**
 * Phase 3: Sidebar Filter UX - E2E Tests
 *
 * Tests the master account sidebar filtering functionality
 * without requiring actual MT4/MT5 environment.
 */

test.describe('Sidebar Filter UX', () => {
  // Setup: Mock API responses
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
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: mockSettings,
          }),
        });
      }
    });

    // Mock WebSocket (prevent connection errors)
    await page.addInitScript(() => {
      class MockWebSocket {
        readyState = 1; // OPEN
        onopen: any = null;
        onmessage: any = null;
        onerror: any = null;
        onclose: any = null;

        constructor(url: string) {
          setTimeout(() => {
            if (this.onopen) this.onopen({});
          }, 10);
        }

        send(data: any) {}
        close() {
          if (this.onclose) this.onclose({});
        }
      }

      (window as any).WebSocket = MockWebSocket;
    });

    // Navigate to the app
    await page.goto('http://localhost:5173');

    // Wait for initial render
    await page.waitForSelector('[aria-label="Master account filter"]', { timeout: 5000 });
  });

  test('should display sidebar with all master accounts', async ({ page }) => {
    // Check that sidebar is visible
    const sidebar = page.locator('[aria-label="Master account filter"]');
    await expect(sidebar).toBeVisible();

    // Check "All Accounts" option
    const allAccountsButton = page.locator('button[role="radio"][aria-checked="true"]').first();
    await expect(allAccountsButton).toContainText('All Accounts');

    // Check that all 3 master accounts are listed
    const masterButtons = page.locator('button[role="radio"]');
    await expect(masterButtons).toHaveCount(4); // 1 "All" + 3 masters

    // Verify master account names
    await expect(page.getByText('Trading Account A')).toBeVisible();
    await expect(page.getByText('Trading Account B')).toBeVisible();
    await expect(page.getByText('Trading Account C')).toBeVisible();
  });

  test('should show connection counts for each master', async ({ page }) => {
    // Account A should have 2 connections
    const accountA = page.locator('button:has-text("Trading Account A")');
    await expect(accountA).toContainText('2 links');

    // Account B should have 1 connection
    const accountB = page.locator('button:has-text("Trading Account B")');
    await expect(accountB).toContainText('1 link'); // singular

    // Account C should have 1 connection
    const accountC = page.locator('button:has-text("Trading Account C")');
    await expect(accountC).toContainText('1 link');
  });

  test('should filter accounts when clicking a master', async ({ page }) => {
    // Initially, all accounts should be visible (3 masters + 4 slaves = 7 cards)
    // Note: Actual count depends on how accounts are rendered
    const initialCards = page.locator('[data-testid="account-card"], .account-card, [class*="AccountCard"]');

    // Click on Trading Account A
    await page.locator('button:has-text("Trading Account A")').click();

    // Wait for filter to apply
    await page.waitForTimeout(500); // Animation duration

    // Check that filter indicator appears
    await expect(page.getByText('Viewing:')).toBeVisible();
    await expect(page.getByText('Trading Account A')).toBeVisible();

    // Check that clear filter button exists
    const clearButton = page.getByRole('button', { name: /clear filter/i });
    await expect(clearButton).toBeVisible();
  });

  test('should clear filter when clicking clear button', async ({ page }) => {
    // Click on a master to filter
    await page.locator('button:has-text("Trading Account A")').click();
    await page.waitForTimeout(300);

    // Verify filter is active
    await expect(page.getByText('Viewing:')).toBeVisible();

    // Click clear filter button
    await page.getByRole('button', { name: /clear filter/i }).click();
    await page.waitForTimeout(300);

    // Verify filter indicator is gone
    await expect(page.getByText('Viewing:')).not.toBeVisible();

    // Verify "All Accounts" is selected
    const allAccountsButton = page.locator('button[role="radio"]:has-text("All Accounts")');
    await expect(allAccountsButton).toHaveAttribute('aria-checked', 'true');
  });

  test('should show online/offline status indicators', async ({ page }) => {
    // Account A is online - should have green indicator
    const accountA = page.locator('button:has-text("Trading Account A")');
    await expect(accountA).toContainText('Online');

    // Account B is offline - should have offline text
    const accountB = page.locator('button:has-text("Trading Account B")');
    await expect(accountB).toContainText('Offline');
  });

  test('should update sidebar selection on click', async ({ page }) => {
    const accountAButton = page.locator('button[role="radio"]:has-text("Trading Account A")');
    const accountBButton = page.locator('button[role="radio"]:has-text("Trading Account B")');

    // Initially, "All Accounts" is selected
    await expect(accountAButton).toHaveAttribute('aria-checked', 'false');

    // Click Account A
    await accountAButton.click();
    await page.waitForTimeout(100);

    // Verify Account A is now selected
    await expect(accountAButton).toHaveAttribute('aria-checked', 'true');

    // Click Account B
    await accountBButton.click();
    await page.waitForTimeout(100);

    // Verify Account B is now selected and A is not
    await expect(accountBButton).toHaveAttribute('aria-checked', 'true');
    await expect(accountAButton).toHaveAttribute('aria-checked', 'false');
  });

  test('should display total connection count in "All Accounts"', async ({ page }) => {
    const allAccountsButton = page.locator('button[role="radio"]:has-text("All Accounts")');

    // Should show total of 4 connections
    await expect(allAccountsButton).toContainText('4');
  });
});

test.describe('Keyboard Navigation', () => {
  test.beforeEach(async ({ page }) => {
    // Same setup as above
    await page.route('**/api/connections', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: mockConnections }),
      });
    });

    await page.route('**/api/settings', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: mockSettings }),
        });
      }
    });

    await page.addInitScript(() => {
      class MockWebSocket {
        readyState = 1;
        onopen: any = null;
        onmessage: any = null;
        onerror: any = null;
        onclose: any = null;
        constructor() { setTimeout(() => { if (this.onopen) this.onopen({}); }, 10); }
        send() {}
        close() { if (this.onclose) this.onclose({}); }
      }
      (window as any).WebSocket = MockWebSocket;
    });

    await page.goto('http://localhost:5173');
    await page.waitForSelector('[aria-label="Master account filter"]');
  });

  test('should navigate with Arrow Down key', async ({ page }) => {
    const allAccountsButton = page.locator('button[role="radio"]:has-text("All Accounts")');

    // Focus on "All Accounts"
    await allAccountsButton.focus();
    await expect(allAccountsButton).toBeFocused();

    // Press Arrow Down
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(50);

    // Next button should be focused (Trading Account A)
    const accountAButton = page.locator('button[role="radio"]:has-text("Trading Account A")');
    await expect(accountAButton).toBeFocused();
  });

  test('should navigate with Arrow Up key', async ({ page }) => {
    const accountAButton = page.locator('button[role="radio"]:has-text("Trading Account A")');

    // Focus on Account A
    await accountAButton.focus();
    await expect(accountAButton).toBeFocused();

    // Press Arrow Up
    await page.keyboard.press('ArrowUp');
    await page.waitForTimeout(50);

    // Should go back to "All Accounts"
    const allAccountsButton = page.locator('button[role="radio"]:has-text("All Accounts")');
    await expect(allAccountsButton).toBeFocused();
  });

  test('should select master with Enter key', async ({ page }) => {
    const accountAButton = page.locator('button[role="radio"]:has-text("Trading Account A")');

    // Focus and press Enter
    await accountAButton.focus();
    await page.keyboard.press('Enter');
    await page.waitForTimeout(300);

    // Should be selected and filter should be active
    await expect(accountAButton).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByText('Viewing:')).toBeVisible();
  });

  test('should select master with Space key', async ({ page }) => {
    const accountBButton = page.locator('button[role="radio"]:has-text("Trading Account B")');

    // Focus and press Space
    await accountBButton.focus();
    await page.keyboard.press('Space');
    await page.waitForTimeout(300);

    // Should be selected
    await expect(accountBButton).toHaveAttribute('aria-checked', 'true');
  });
});

test.describe('Mobile Drawer (viewport < 1024px)', () => {
  test.use({ viewport: { width: 375, height: 667 } }); // iPhone SE size

  test.beforeEach(async ({ page }) => {
    await page.route('**/api/connections', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: mockConnections }),
      });
    });

    await page.route('**/api/settings', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: mockSettings }),
        });
      }
    });

    await page.addInitScript(() => {
      class MockWebSocket {
        readyState = 1;
        onopen: any = null;
        onmessage: any = null;
        onerror: any = null;
        onclose: any = null;
        constructor() { setTimeout(() => { if (this.onopen) this.onopen({}); }, 10); }
        send() {}
        close() { if (this.onclose) this.onclose({}); }
      }
      (window as any).WebSocket = MockWebSocket;
    });

    await page.goto('http://localhost:5173');
  });

  test('should show hamburger menu button on mobile', async ({ page }) => {
    // Look for button with "Filter Accounts" text or Menu icon
    const menuButton = page.getByRole('button', { name: /filter accounts/i });
    await expect(menuButton).toBeVisible();
  });

  test('should open drawer when clicking hamburger menu', async ({ page }) => {
    // Click menu button
    const menuButton = page.getByRole('button', { name: /filter accounts/i });
    await menuButton.click();
    await page.waitForTimeout(500); // Animation

    // Drawer should be visible
    const drawer = page.locator('[aria-label="Master account filter"]');
    await expect(drawer).toBeVisible();
  });

  test('should close drawer when clicking backdrop', async ({ page }) => {
    // Open drawer
    const menuButton = page.getByRole('button', { name: /filter accounts/i });
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click backdrop (outside drawer)
    await page.locator('.fixed.inset-0.bg-black\\/50').click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(300);

    // Drawer should be hidden
    const drawer = page.locator('[aria-label="Master account filter"]');
    await expect(drawer).not.toBeVisible();
  });

  test('should close drawer after selecting a master', async ({ page }) => {
    // Open drawer
    const menuButton = page.getByRole('button', { name: /filter accounts/i });
    await menuButton.click();
    await page.waitForTimeout(300);

    // Click a master account
    await page.locator('button[role="radio"]:has-text("Trading Account A")').click();
    await page.waitForTimeout(500);

    // Drawer should close automatically
    const drawer = page.locator('[aria-label="Master account filter"]');
    await expect(drawer).not.toBeVisible();

    // Filter should be applied
    await expect(page.getByText('Viewing:')).toBeVisible();
  });
});

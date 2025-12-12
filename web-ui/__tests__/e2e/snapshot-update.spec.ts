import { test, expect } from '@playwright/test';

// Define types locally for the test
interface SystemStateSnapshot {
  connections: any[];
  trade_groups: any[];
  members: any[];
}

test('UI updates immediately when system_snapshot is received', async ({ page }) => {
  // Mock WebSocket server
  // We'll intercept the WebSocket construction and mock the behavior
  await page.addInitScript(() => {
    // Save original WebSocket
    const OriginalWebSocket = window.WebSocket;

    // Create a mock WebSocket
    class MockWebSocket extends EventTarget {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;

      readyState: number = MockWebSocket.CONNECTING;
      url: string;
      onopen: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;

      constructor(url: string) {
        super();
        this.url = url;

        // Expose this instance globally so the test can control it
        (window as any).__mockWebSocket = this;

        setTimeout(() => {
          this.readyState = MockWebSocket.OPEN;
          this.dispatchEvent(new Event('open'));
          if (this.onopen) this.onopen(new Event('open'));
          console.log('[MockWS] Connected');
        }, 100);
      }

      send(data: any) {
        console.log('[MockWS] Client sent:', data);
      }

      close() {
        this.readyState = MockWebSocket.CLOSED;
      }
    }

    // Override global WebSocket
    (window as any).WebSocket = MockWebSocket;
  });

  // Navigate to page
  // Mock API responses first to ensure page loads even if backend is offline
  await page.route('/api/connections', async route => {
    await route.fulfill({ json: [] });
  });
  await page.route('/api/trade-groups', async route => {
    await route.fulfill({ json: [] });
  });

  await page.goto('/ja/connections');

  // Wait for React Flow to initialize
  await page.waitForTimeout(1000);

  // Define initial snapshot data (Clean state)
  const initialSnapshot: SystemStateSnapshot = {
    connections: [
      {
        account_id: 'Master_1',
        ea_type: 'Master',
        status: 'Online',
        is_trade_allowed: true, // AutoTrading ON
        platform: 'MT5',
        account_number: 123456,
        broker: 'Test Broker',
        account_name: 'Master 1',
        server: 'Demo',
        balance: 10000,
        equity: 10000,
        currency: 'USD',
        leverage: 100,
        last_heartbeat: new Date().toISOString(),
        connected_at: new Date().toISOString(),
      }
    ],
    trade_groups: [
      {
        id: 'Master_1',
        master_settings: { enabled: true, config_version: 1 },
        master_runtime_status: 2, // CONNECTED
        master_warning_codes: [],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }
    ],
    members: []
  };

  // Send initial snapshot via mock WS
  await page.evaluate((snapshot) => {
    const ws = (window as any).__mockWebSocket;
    if (ws && ws.onmessage) {
      ws.onmessage({ data: `system_snapshot:${JSON.stringify(snapshot)}` });
    }
  }, initialSnapshot);

  // Verify Master Node appears and is Green (No warning)
  // Master node ID in React Flow is `source-Master_1`
  await expect(page.locator('[data-testid="account-node"][data-account-id="Master_1"]')).toBeVisible();
  // Check for absence of warning banner
  await expect(page.locator('[data-testid="account-node"][data-account-id="Master_1"] .text-yellow-600')).not.toBeVisible();

  // Simulate Master AutoTrading OFF event (via new snapshot)
  const autoTradingOffSnapshot: SystemStateSnapshot = {
    connections: [
      {
        ...initialSnapshot.connections[0],
        is_trade_allowed: false // AutoTrading OFF
      }
    ],
    trade_groups: [
      {
        ...initialSnapshot.trade_groups[0],
        master_runtime_status: 0, // DISABLED
        master_warning_codes: ['master_auto_trading_disabled']
      }
    ],
    members: []
  };

  // Send update snapshot
  await page.evaluate((snapshot) => {
    const ws = (window as any).__mockWebSocket;
    if (ws && ws.onmessage) {
      ws.onmessage({ data: `system_snapshot:${JSON.stringify(snapshot)}` });
    }
  }, autoTradingOffSnapshot);

  // Verify Warning appears immediately
  // Warning text for 'master_auto_trading_disabled' should be present
  // Note: content is localized, checking for warning icon or container class
  await expect(page.locator('[data-testid="account-node"][data-account-id="Master_1"] .text-yellow-600')).toBeVisible();

  // Optionally verify text if we know the locale (ja)
  // "自動売買が無効です" (AutoTrading is disabled)
  await expect(page.locator('[data-testid="account-node"][data-account-id="Master_1"]')).toContainText('自動売買が無効');

  // Verify Master Node status indicator changed (Green -> Yellow/Gray)
  // The handle color class changes
  const handle = page.locator('[data-testid="account-node"][data-account-id="Master_1"] .react-flow__handle');
  await expect(handle).toHaveClass(/bg-yellow-500/);

});

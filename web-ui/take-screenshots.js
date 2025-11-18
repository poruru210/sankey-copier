const { chromium } = require('@playwright/test');

async function takeScreenshots() {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
  });

  try {
    // Desktop screenshot
    console.log('Taking desktop screenshot...');
    const desktopContext = await browser.newContext({
      viewport: { width: 1920, height: 1080 },
      deviceScaleFactor: 1,
    });
    const desktopPage = await desktopContext.newPage();

    // Mock APIs
    await desktopPage.route('**/api/connections', async (route) => {
      const mockConnections = [
        {
          account_id: 'FxPro_12345001',
          ea_type: 'Master',
          platform: 'MT5',
          account_number: 12345001,
          broker: 'FxPro',
          account_name: 'Trading Account A',
          server: 'FxPro-Live',
          balance: 10000,
          equity: 10500,
          currency: 'USD',
          leverage: 500,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 3,
        },
        {
          account_id: 'OANDA_67890002',
          ea_type: 'Master',
          platform: 'MT5',
          account_number: 67890002,
          broker: 'OANDA',
          account_name: 'Trading Account B',
          server: 'OANDA-Live',
          balance: 5000,
          equity: 5200,
          currency: 'USD',
          leverage: 100,
          last_heartbeat: new Date(Date.now() - 10 * 60 * 1000).toISOString(),
          status: 'Offline',
          connected_at: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
          open_positions: 1,
        },
        {
          account_id: 'XM_11111003',
          ea_type: 'Master',
          platform: 'MT5',
          account_number: 11111003,
          broker: 'XM',
          account_name: 'Demo Account C',
          server: 'XM-Demo',
          balance: 100000,
          equity: 99800,
          currency: 'USD',
          leverage: 888,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 0,
        },
        {
          account_id: 'FxPro_22222004',
          ea_type: 'Slave',
          platform: 'MT5',
          account_number: 22222004,
          broker: 'FxPro',
          account_name: 'Copy Account 1',
          server: 'FxPro-Live',
          balance: 1000,
          equity: 1050,
          currency: 'USD',
          leverage: 500,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 2,
        },
        {
          account_id: 'FxPro_33333005',
          ea_type: 'Slave',
          platform: 'MT5',
          account_number: 33333005,
          broker: 'FxPro',
          account_name: 'Copy Account 2',
          server: 'FxPro-Live',
          balance: 2000,
          equity: 2100,
          currency: 'USD',
          leverage: 500,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 3,
        },
      ];
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: mockConnections }),
      });
    });

    await desktopPage.route('**/api/settings', async (route) => {
      if (route.request().method() === 'GET') {
        const mockSettings = [
          {
            id: 1,
            status: 2, // STATUS_CONNECTED
            master_account: 'FxPro_12345001',
            slave_account: 'FxPro_22222004',
            lot_multiplier: 1.5,
            reverse_trade: false,
            symbol_filters: [],
            magic_number_filters: [],
            comment_filters: [],
            symbol_mappings: [],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
          {
            id: 2,
            status: 2, // STATUS_CONNECTED
            master_account: 'FxPro_12345001',
            slave_account: 'FxPro_33333005',
            lot_multiplier: 0.5,
            reverse_trade: false,
            symbol_filters: [],
            magic_number_filters: [],
            comment_filters: [],
            symbol_mappings: [],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        ];
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: mockSettings }),
        });
      }
    });

    // Mock WebSocket
    await desktopPage.addInitScript(() => {
      class MockWebSocket {
        readyState = 1; // OPEN
        onopen = null;
        onclose = null;
        constructor() {
          setTimeout(() => {
            if (this.onopen) this.onopen({});
          }, 10);
        }
        send() {}
        close() {
          if (this.onclose) this.onclose({});
        }
      }
      window.WebSocket = MockWebSocket;
    });

    await desktopPage.goto('http://localhost:5173');
    await desktopPage.waitForSelector('[aria-label="Master account filter"]', { timeout: 10000 });
    await desktopPage.waitForTimeout(1000); // Wait for animations
    await desktopPage.screenshot({ path: '/home/user/sankey-copier/screenshot-desktop.png', fullPage: true });
    console.log('Desktop screenshot saved to screenshot-desktop.png');

    await desktopContext.close();

    // Mobile screenshot
    console.log('Taking mobile screenshot...');
    const mobileContext = await browser.newContext({
      viewport: { width: 375, height: 667 },
      deviceScaleFactor: 2,
      isMobile: true,
      hasTouch: true,
    });
    const mobilePage = await mobileContext.newPage();

    // Mock APIs for mobile
    await mobilePage.route('**/api/connections', async (route) => {
      const mockConnections = [
        {
          account_id: 'FxPro_12345001',
          ea_type: 'Master',
          platform: 'MT5',
          account_number: 12345001,
          broker: 'FxPro',
          account_name: 'Trading Account A',
          server: 'FxPro-Live',
          balance: 10000,
          equity: 10500,
          currency: 'USD',
          leverage: 500,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 3,
        },
        {
          account_id: 'OANDA_67890002',
          ea_type: 'Master',
          platform: 'MT5',
          account_number: 67890002,
          broker: 'OANDA',
          account_name: 'Trading Account B',
          server: 'OANDA-Live',
          balance: 5000,
          equity: 5200,
          currency: 'USD',
          leverage: 100,
          last_heartbeat: new Date(Date.now() - 10 * 60 * 1000).toISOString(),
          status: 'Offline',
          connected_at: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
          open_positions: 1,
        },
        {
          account_id: 'FxPro_22222004',
          ea_type: 'Slave',
          platform: 'MT5',
          account_number: 22222004,
          broker: 'FxPro',
          account_name: 'Copy Account 1',
          server: 'FxPro-Live',
          balance: 1000,
          equity: 1050,
          currency: 'USD',
          leverage: 500,
          last_heartbeat: new Date().toISOString(),
          status: 'Online',
          connected_at: new Date().toISOString(),
          open_positions: 2,
        },
      ];
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: mockConnections }),
      });
    });

    await mobilePage.route('**/api/settings', async (route) => {
      if (route.request().method() === 'GET') {
        const mockSettings = [
          {
            id: 1,
            status: 2, // STATUS_CONNECTED
            master_account: 'FxPro_12345001',
            slave_account: 'FxPro_22222004',
            lot_multiplier: 1.5,
            reverse_trade: false,
            symbol_filters: [],
            magic_number_filters: [],
            comment_filters: [],
            symbol_mappings: [],
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        ];
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: mockSettings }),
        });
      }
    });

    // Mock WebSocket for mobile
    await mobilePage.addInitScript(() => {
      class MockWebSocket {
        readyState = 1; // OPEN
        onopen = null;
        onclose = null;
        constructor() {
          setTimeout(() => {
            if (this.onopen) this.onopen({});
          }, 10);
        }
        send() {}
        close() {
          if (this.onclose) this.onclose({});
        }
      }
      window.WebSocket = MockWebSocket;
    });

    await mobilePage.goto('http://localhost:5173');
    await mobilePage.waitForSelector('button[aria-label*="menu"]', { timeout: 10000 });
    await mobilePage.waitForTimeout(1000);
    await mobilePage.screenshot({ path: '/home/user/sankey-copier/screenshot-mobile.png', fullPage: true });
    console.log('Mobile screenshot saved to screenshot-mobile.png');

    // Take screenshot with drawer open
    console.log('Taking mobile screenshot with drawer open...');
    await mobilePage.click('button[aria-label*="menu"]');
    await mobilePage.waitForTimeout(500); // Wait for drawer animation
    await mobilePage.screenshot({ path: '/home/user/sankey-copier/screenshot-mobile-drawer.png', fullPage: true });
    console.log('Mobile drawer screenshot saved to screenshot-mobile-drawer.png');

    await mobileContext.close();

  } catch (error) {
    console.error('Error taking screenshots:', error);
    throw error;
  } finally {
    await browser.close();
  }
}

takeScreenshots().catch(console.error);

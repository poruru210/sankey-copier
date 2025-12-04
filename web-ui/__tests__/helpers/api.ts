import type { Page, Route } from '@playwright/test';
import {
  mockConnections,
  mockTradeGroups,
  mockTradeGroupMembers,
  mockVictoriaLogsConfig,
  mockVictoriaLogsSettings,
  mockServerLogs,
} from '../mocks/testData';

const shouldLogApi = process.env.PLAYWRIGHT_LOG_API === '1';

function extractPathSegment(url: string, marker: string): string {
  const pathname = new URL(url).pathname;
  const segments = pathname.split('/').filter(Boolean);
  const markerIndex = segments.findIndex((segment) => segment === marker);
  if (markerIndex === -1 || markerIndex + 1 >= segments.length) {
    return '';
  }
  return decodeURIComponent(segments[markerIndex + 1]);
}

async function fulfillJson(route: Route, data: unknown, status = 200): Promise<void> {
  await route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(data),
  });
}

export async function setupDefaultApiMocks(page: Page): Promise<void> {
  if (shouldLogApi) {
    console.log('[MockAPI] installing default routes');
    page.on('request', (request) => {
      if (request.url().includes('/api/')) {
        console.log('[MockAPI] request issued', request.method(), request.url());
      }
    });
    page.on('requestfailed', (request) => {
      if (request.url().includes('/api/')) {
        console.log('[MockAPI] request failed', request.url(), request.failure());
      }
    });
    page.on('console', (message) => {
      if (message.type() === 'error' || message.text().includes('/api/')) {
        console.log('[PAGE console]', message.type(), message.text());
      }
    });
    page.on('pageerror', (error) => {
      console.log('[PAGE error event]', error.message);
    });
  }
  await page.route('**/api/connections', async (route) => {
    if (shouldLogApi) {
      console.log('[MockAPI] connections', route.request().url());
    }
    await fulfillJson(route, mockConnections);
  });

  await page.route('**/api/trade-groups', async (route) => {
    if (shouldLogApi) {
      console.log('[MockAPI] trade-groups list', route.request().url());
    }
    await fulfillJson(route, mockTradeGroups);
  });

  await page.route('**/api/trade-groups/*/members', async (route) => {
    const masterAccount = extractPathSegment(route.request().url(), 'trade-groups');
    const members = mockTradeGroupMembers[masterAccount] ?? [];
    if (shouldLogApi) {
      console.log('[MockAPI] trade-groups members', masterAccount, '->', members.length);
    }
    await fulfillJson(route, members);
  });

  await page.route('**/api/victoria-logs-config', async (route) => {
    if (shouldLogApi) {
      console.log('[MockAPI] victoria-logs-config', route.request().url());
    }
    await fulfillJson(route, mockVictoriaLogsConfig);
  });

  await page.route('**/api/victoria-logs-settings', async (route) => {
    const method = route.request().method();
    if (shouldLogApi) {
      console.log('[MockAPI] victoria-logs-settings', method, route.request().url());
    }
    if (method === 'GET') {
      await fulfillJson(route, mockVictoriaLogsSettings);
    } else {
      await fulfillJson(route, { ok: true });
    }
  });

  await page.route('**/api/logs', async (route) => {
    if (shouldLogApi) {
      console.log('[MockAPI] server-logs', route.request().url());
    }
    await fulfillJson(route, mockServerLogs);
  });
}

export async function installBasicWebSocketMock(page: Page): Promise<void> {
  await page.addInitScript(() => {
    class MockWebSocket {
      static sockets: MockWebSocket[] = [];
      readyState = 0;
      onopen: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;

      constructor(public url: string) {
        MockWebSocket.sockets.push(this);
        setTimeout(() => {
          this.readyState = 1;
          this.onopen?.(new Event('open'));
        }, 10);
      }

      send(): void {}

      close(): void {
        this.readyState = 3;
        this.onclose?.(new CloseEvent('close'));
      }
    }

    (window as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;
  });
}

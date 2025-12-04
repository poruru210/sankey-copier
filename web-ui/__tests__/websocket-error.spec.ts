import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers/navigation';
import { setupDefaultApiMocks, installBasicWebSocketMock } from './helpers/api';

test('Check WebSocket connection errors in console', async ({ page }) => {
  const consoleMessages: string[] = [];
  const errors: string[] = [];

  // Capture console messages
  page.on('console', msg => {
    const text = msg.text();
    consoleMessages.push(text);
    if (text.includes('WebSocket')) {
      console.log('WebSocket log:', text);
    }
  });

  // Capture page errors
  page.on('pageerror', error => {
    errors.push(error.message);
    if (error.message.includes('WebSocket')) {
      console.log('WebSocket error:', error.message);
    }
  });

  await setupDefaultApiMocks(page);
  await installBasicWebSocketMock(page);

  // Navigate to the app
  await gotoApp(page, '/en/connections');

  // Wait for page to load
  await page.waitForTimeout(3000);

  // Check for WebSocket errors
  const hasWebSocketError = errors.some(err =>
    err.includes('WebSocket is closed before the connection is established')
  );

  const hasWebSocketLog = consoleMessages.some(msg =>
    msg.includes('WebSocket connecting') || msg.includes('WebSocket connected')
  );

  console.log('\n=== Test Results ===');
  console.log('Has WebSocket connection logs:', hasWebSocketLog);
  console.log('Has WebSocket error:', hasWebSocketError);
  console.log('\nAll console messages:', consoleMessages.filter(msg => msg.includes('WebSocket')));
  console.log('\nAll errors:', errors.filter(err => err.includes('WebSocket')));

  // Assertion: WebSocket should connect without errors
  expect(hasWebSocketError).toBe(false);
  expect(hasWebSocketLog).toBe(true);
});

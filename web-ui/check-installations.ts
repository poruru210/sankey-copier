import { chromium } from '@playwright/test';

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();

  try {
    console.log('Navigating to /en/installations...');
    await page.goto('http://localhost:5173/en/installations', { waitUntil: 'networkidle' });

    // Take screenshot
    await page.screenshot({ path: 'installations-page.png', fullPage: true });
    console.log('Screenshot saved to installations-page.png');

    // Get page title
    const title = await page.title();
    console.log('Page title:', title);

    // Check for any error messages
    const errorElements = await page.locator('[role="alert"], .error, .destructive').all();
    console.log('Number of error elements:', errorElements.length);

    // Check if loading indicator is present
    const loadingIndicator = await page.locator('text=Loading installations').count();
    console.log('Loading indicator present:', loadingIndicator > 0);

    // Check if "No running MT4/MT5 installations detected" message is present
    const noInstallsMessage = await page.locator('text=No running MT4/MT5 installations detected').count();
    console.log('No installations message present:', noInstallsMessage > 0);

    // Get all visible text on page
    const bodyText = await page.locator('body').textContent();
    console.log('\n=== Page Content ===');
    console.log(bodyText?.slice(0, 500)); // First 500 characters

    // Check network requests
    const apiRequests: string[] = [];
    page.on('request', request => {
      if (request.url().includes('/api/')) {
        apiRequests.push(request.url());
      }
    });

    await page.waitForTimeout(2000);
    console.log('\n=== API Requests ===');
    console.log(apiRequests);

  } catch (error) {
    console.error('Error:', error);
  } finally {
    await browser.close();
  }
})();

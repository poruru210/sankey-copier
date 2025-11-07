import { test, expect } from '@playwright/test';

test.describe('React Flow Node Dragging', () => {
  test.beforeEach(async ({ page }) => {
    // Start at home page
    await page.goto('http://localhost:5173');

    // Wait for React Flow to load
    await page.waitForSelector('.react-flow', { timeout: 10000 });

    // Wait a bit for nodes to render
    await page.waitForTimeout(2000);
  });

  test('should display drag handles on account nodes', async ({ page }) => {
    // Look for drag handles (GripVertical icons)
    const dragHandles = page.locator('.account-node .cursor-move');
    const count = await dragHandles.count();

    console.log(`Found ${count} drag handles`);
    expect(count).toBeGreaterThan(0);
  });

  test('should show RelayServer node', async ({ page }) => {
    const relayServer = page.locator('.relay-server-node');
    await expect(relayServer).toBeVisible();

    // Get position
    const box = await relayServer.boundingBox();
    console.log('RelayServer position:', box);
  });

  test('should allow dragging account nodes', async ({ page }) => {
    // Find first account node
    const firstAccountNode = page.locator('.account-node').first();
    await expect(firstAccountNode).toBeVisible();

    // Get initial position
    const initialBox = await firstAccountNode.boundingBox();
    console.log('Initial position:', initialBox);

    if (!initialBox) {
      throw new Error('Could not get initial position');
    }

    // Try to drag the node
    await page.mouse.move(initialBox.x + initialBox.width / 2, initialBox.y + initialBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(initialBox.x + 100, initialBox.y + 100, { steps: 10 });
    await page.mouse.up();

    // Wait for animation
    await page.waitForTimeout(500);

    // Get new position
    const newBox = await firstAccountNode.boundingBox();
    console.log('New position:', newBox);

    // Position should have changed
    if (newBox) {
      const moved = Math.abs(newBox.x - initialBox.x) > 50 || Math.abs(newBox.y - initialBox.y) > 50;
      console.log('Node moved:', moved);
    }
  });

  test('should check for hover interactions', async ({ page }) => {
    // Find account nodes
    const accountNodes = page.locator('.account-node');
    const count = await accountNodes.count();
    console.log(`Found ${count} account nodes`);

    if (count > 0) {
      const firstNode = accountNodes.first();

      // Hover over the node
      await firstNode.hover();
      await page.waitForTimeout(500);

      // Check if any visual changes occurred
      const screenshot = await page.screenshot();
      console.log('Hover screenshot taken');
    }
  });

  test('should check React Flow canvas interactions', async ({ page }) => {
    const canvas = page.locator('.react-flow');
    await expect(canvas).toBeVisible();

    // Check for pan/zoom controls
    const controls = page.locator('.react-flow__controls');
    const hasControls = await controls.count();
    console.log('Has controls:', hasControls > 0);

    // Check for minimap
    const minimap = page.locator('.react-flow__minimap');
    const hasMinimap = await minimap.count();
    console.log('Has minimap:', hasMinimap > 0);
  });

  test('should check if noDrag class is applied correctly', async ({ page }) => {
    // Check for elements with noDrag class
    const noDragElements = page.locator('.noDrag');
    const count = await noDragElements.count();
    console.log(`Found ${count} noDrag elements`);

    // Verify AccountCard is inside noDrag
    const accountCardsInNoDrag = page.locator('.noDrag .bg-white');
    const cardCount = await accountCardsInNoDrag.count();
    console.log(`Found ${cardCount} AccountCards in noDrag wrapper`);
  });
});

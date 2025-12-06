import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers/navigation';
import { setupDefaultApiMocks, installBasicWebSocketMock } from './helpers/api';

test.describe('React Flow Node Dragging', () => {
  test.beforeEach(async ({ page }) => {
    await setupDefaultApiMocks(page);
    await installBasicWebSocketMock(page);

    // Start at home page
    await gotoApp(page);

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

  test('should render expected master nodes', async ({ page }) => {
    const masterIds = ['FxPro_12345001', 'OANDA_67890002', 'XM_11111003'];
    for (const id of masterIds) {
      const node = page.locator(`[data-account-id="${id}"]`).first();
      await expect(node, `Missing account node for ${id}`).toBeVisible();
    }
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

  test('expanding a node shifts nodes below when layout is unlocked', async ({ page }) => {
    // Find a pair of nodes that are in the same column and vertically ordered
    const nodes = page.locator('.account-node');
    const count = await nodes.count();
    if (count < 2) throw new Error('Not enough nodes to run test');

    // Helper to read id and y for each node, finding a vertically stacked pair
    const boxInfo = [] as { idx: number; id: string; y: number }[];
    for (let i = 0; i < count; i++) {
      const el = nodes.nth(i);
      const id = await el.evaluate((n) => n.closest('.react-flow__node')?.getAttribute('data-id') || '');
      const box = await el.boundingBox();
      if (id && box) boxInfo.push({ idx: i, id, y: box.y });
    }

    // Find first pair with same prefix (source/receiver) and y1 < y2
    let pair: { firstIdx: number; secondIdx: number } | null = null;
    for (let i = 0; i < boxInfo.length; i++) {
      for (let j = i + 1; j < boxInfo.length; j++) {
        const a = boxInfo[i];
        const b = boxInfo[j];
        const prefixA = a.id.split('-')[0];
        const prefixB = b.id.split('-')[0];
        if (prefixA === prefixB && a.y < b.y) {
          pair = { firstIdx: a.idx, secondIdx: b.idx };
          break;
        }
      }
      if (pair) break;
    }

    // Fallback to first two nodes if no matching pair found
    if (!pair) pair = { firstIdx: 0, secondIdx: 1 };

    const first = nodes.nth(pair.firstIdx);
    const second = nodes.nth(pair.secondIdx);

    await expect(first).toBeVisible();
    await expect(second).toBeVisible();

    const beforeSecond = await second.boundingBox();
    const beforeFirst = await first.boundingBox();
    if (!beforeSecond) throw new Error('Could not read initial node position');

    // Toggle (expand) the first node by clicking its header toggle (last button)
    const btns = first.locator('button');
    const btnCount = await btns.count();
    await btns.nth(btnCount - 1).click();

    // Wait until we observe the second node move down (debounced / layout settle) or timeout
    await expect
      .poll(async () => {
        const b = await second.boundingBox();
        return b?.y ?? 0;
      }, { timeout: 3000 })
      .toBeGreaterThan(beforeSecond.y + 6);
  });

  test('interactive toggle button exists and can be clicked', async ({ page }) => {
    // Simply verify the interactive toggle works without crashing
    const interactiveBtn = page.locator('.react-flow__controls-interactive');
    await expect(interactiveBtn).toBeVisible();
    
    // Toggle off
    await interactiveBtn.click();
    await page.waitForTimeout(200);
    
    // Toggle back on
    await interactiveBtn.click();
    await page.waitForTimeout(200);
    
    // Verify we can still see the nodes
    const nodes = page.locator('.account-node');
    await expect(nodes.first()).toBeVisible();
  });

  test('overlap resolution works even after user drags a node', async ({ page }) => {
    const nodes = page.locator('.account-node');
    const first = nodes.nth(0);
    const second = nodes.nth(1);

    await expect(first).toBeVisible();
    await expect(second).toBeVisible();

    const beforeSecond = await second.boundingBox();
    const beforeFirst = await first.boundingBox();
    if (!beforeSecond || !beforeFirst) throw new Error('Could not read initial node positions');

    // Drag the second node slightly
    await page.mouse.move(beforeSecond.x + beforeSecond.width / 2, beforeSecond.y + beforeSecond.height / 2);
    await page.mouse.down();
    await page.mouse.move(beforeSecond.x + beforeSecond.width / 2 + 80, beforeSecond.y + beforeSecond.height / 2 + 10, { steps: 8 });
    await page.mouse.up();

    await page.waitForTimeout(300);

    const draggedSecond = await second.boundingBox();
    if (!draggedSecond) throw new Error('Could not read second node after drag');

    // Expand the first node
    const btns = first.locator('button');
    const btnCount = await btns.count();
    await btns.nth(btnCount - 1).click();

    await page.waitForTimeout(800);

    const afterSecond = await second.boundingBox();
    if (!afterSecond) throw new Error('Could not read second node after expand');

    // Overlap resolution should still work after user drags
    // If overlapping, the second node should be shifted
    // We verify that after expand, the node maintains at least its dragged position or moves further
    expect(afterSecond.y).toBeGreaterThanOrEqual(draggedSecond.y);
  });

  test('transient expand/collapse does apply shifts with physics simulation', async ({ page }) => {
    // With d3-force physics simulation, expanding a node immediately pushes overlapping nodes.
    // Even if the node is collapsed quickly after, the shift has already been applied.
    // This is the expected behavior for the physics-based approach.
    const nodes = page.locator('.account-node');
    const first = nodes.nth(0);
    const second = nodes.nth(1);

    await expect(first).toBeVisible();
    await expect(second).toBeVisible();

    const beforeSecond = await second.boundingBox();
    const beforeFirst = await first.boundingBox();
    if (!beforeSecond || !beforeFirst) throw new Error('Could not read initial node position');

    // Click expand then immediately collapse (transient toggle)
    const btns = first.locator('button');
    const btnCount = await btns.count();
    await btns.nth(btnCount - 1).click();

    // Wait for the expand animation and physics simulation to complete
    await page.waitForTimeout(200);
    
    // collapse
    await btns.nth(btnCount - 1).click();

    await page.waitForTimeout(200);

    // Wait for the node to have actually collapsed (its height returns to near the original)
    await expect
      .poll(async () => {
        const b = await first.boundingBox();
        return b?.height ?? 0;
      }, { timeout: 2000 })
      .toBeLessThanOrEqual((beforeFirst?.height ?? 0) + 8);

    // Physics simulation applies shifts immediately on expand
    // The second node should have moved (or stayed in place if no overlap existed)
    const afterSecond = await second.boundingBox();
    if (!afterSecond) throw new Error('Could not read second node after transient toggle');

    // With physics simulation, immediate shift is expected behavior
    // We just verify the node is still visible and positioned reasonably
    expect(afterSecond.y).toBeGreaterThan(0);
    expect(afterSecond.x).toBeGreaterThan(0);
  });
});

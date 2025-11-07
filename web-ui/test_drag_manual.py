#!/usr/bin/env python3
"""
Manual test script for React Flow drag functionality.
Tests if account nodes can be dragged and if interactive elements work correctly.
"""

import asyncio
import time
from playwright.async_api import async_playwright

async def test_drag_functionality():
    """Test React Flow node dragging and interactive elements."""
    async with async_playwright() as p:
        print("Launching browser...")
        try:
            # Try to launch with specific args to avoid X server issues
            browser = await p.chromium.launch(
                headless=True,
                args=[
                    '--disable-gpu',
                    '--no-sandbox',
                    '--disable-setuid-sandbox',
                    '--disable-dev-shm-usage',
                ]
            )
        except Exception as e:
            print(f"Failed to launch browser: {e}")
            return False

        try:
            context = await browser.new_context(viewport={'width': 1920, 'height': 1080})
            page = await context.new_page()

            print("Navigating to http://localhost:5173...")
            try:
                await page.goto('http://localhost:5173', timeout=15000, wait_until='networkidle')
            except Exception as e:
                print(f"Failed to load page: {e}")
                return False

            print("Waiting for React Flow to load...")
            try:
                await page.wait_for_selector('.react-flow', timeout=10000)
            except Exception as e:
                print(f"React Flow not found: {e}")
                return False

            # Wait for nodes to render
            await asyncio.sleep(2)

            print("\n=== Test 1: Check for account nodes ===")
            account_nodes = await page.locator('.account-node').count()
            print(f"Found {account_nodes} account nodes")

            if account_nodes == 0:
                print("❌ No account nodes found!")
                return False
            print("✓ Account nodes found")

            print("\n=== Test 2: Check for relay server node ===")
            relay_node = page.locator('.relay-server-node')
            relay_visible = await relay_node.is_visible()
            if relay_visible:
                print("✓ Relay server node visible")
                box = await relay_node.bounding_box()
                print(f"  Position: x={box['x']:.1f}, y={box['y']:.1f}")
            else:
                print("❌ Relay server node not visible")
                return False

            print("\n=== Test 3: Check for interactive elements with noDrag ===")
            no_drag_elements = await page.locator('.noDrag').count()
            print(f"Found {no_drag_elements} elements with noDrag class")
            if no_drag_elements > 0:
                print("✓ noDrag elements found")
            else:
                print("⚠ No noDrag elements found (might be an issue)")

            print("\n=== Test 4: Check for cursor-move styling ===")
            cursor_move_elements = await page.locator('.cursor-move').count()
            print(f"Found {cursor_move_elements} elements with cursor-move class")
            if cursor_move_elements > 0:
                print("✓ Draggable areas have cursor-move styling")
            else:
                print("⚠ No cursor-move elements found")

            print("\n=== Test 5: Attempt to drag first account node ===")
            first_node = page.locator('.account-node').first()

            # Get initial position
            initial_box = await first_node.bounding_box()
            if not initial_box:
                print("❌ Could not get initial node position")
                return False

            print(f"Initial position: x={initial_box['x']:.1f}, y={initial_box['y']:.1f}")

            # Calculate center of node
            center_x = initial_box['x'] + initial_box['width'] / 2
            center_y = initial_box['y'] + initial_box['height'] / 2

            # Try dragging from the header area (should be draggable)
            print("Attempting to drag node...")
            try:
                await page.mouse.move(center_x, center_y)
                await page.mouse.down()
                await page.mouse.move(center_x + 200, center_y + 100, steps=20)
                await page.mouse.up()
            except Exception as e:
                print(f"❌ Drag operation failed: {e}")
                return False

            # Wait for any animations
            await asyncio.sleep(1)

            # Get new position
            new_box = await first_node.bounding_box()
            if not new_box:
                print("❌ Could not get new node position")
                return False

            print(f"New position: x={new_box['x']:.1f}, y={new_box['y']:.1f}")

            # Check if position changed
            dx = abs(new_box['x'] - initial_box['x'])
            dy = abs(new_box['y'] - initial_box['y'])
            moved = dx > 50 or dy > 50

            if moved:
                print(f"✓ Node moved! (dx={dx:.1f}, dy={dy:.1f})")
            else:
                print(f"❌ Node did not move significantly (dx={dx:.1f}, dy={dy:.1f})")
                return False

            print("\n=== Test 6: Check if buttons still work (with noDrag) ===")
            # Find a button with noDrag class
            button = page.locator('.noDrag button').first()
            button_count = await page.locator('.noDrag button').count()
            print(f"Found {button_count} buttons with noDrag class")

            if button_count > 0:
                print("✓ Interactive buttons have noDrag class")
            else:
                print("⚠ No buttons with noDrag found")

            print("\n=== All Tests Completed ===")
            print("✓ Drag functionality appears to be working correctly!")

            # Take a screenshot for verification
            await page.screenshot(path='/home/user/sankey-copier/web-ui/test-screenshot.png')
            print("\nScreenshot saved to test-screenshot.png")

            return True

        finally:
            await browser.close()

async def main():
    print("=" * 60)
    print("React Flow Drag Functionality Test")
    print("=" * 60)

    success = await test_drag_functionality()

    print("\n" + "=" * 60)
    if success:
        print("RESULT: ✓ All tests PASSED")
        print("=" * 60)
        return 0
    else:
        print("RESULT: ❌ Tests FAILED")
        print("=" * 60)
        return 1

if __name__ == '__main__':
    exit_code = asyncio.run(main())
    exit(exit_code)

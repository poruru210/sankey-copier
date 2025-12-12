from playwright.sync_api import Page, expect, sync_playwright
import time

def test_resize_fit_view(page: Page):
    # Enable console logging
    page.on("console", lambda msg: print(f"Browser Console: {msg.text}"))

    # 1. Arrange: Go to the main page where ConnectionsViewReactFlow is likely rendered.
    print("Navigating to http://localhost:8080")
    page.goto("http://localhost:8080", timeout=60000)

    # Wait for the flow to load (nodes to appear).
    print("Waiting for .react-flow__node...")
    # Wait up to 30 seconds for the nodes to appear (accounting for mock delay)
    page.wait_for_selector(".react-flow__node", timeout=30000)

    # Initial screenshot
    print("Taking initial screenshot...")
    page.screenshot(path="verification/before_resize.png")

    # 2. Act: Resize the window.
    print("Resizing viewport to 800x600...")
    page.set_viewport_size({"width": 800, "height": 600})

    # 3. Wait: Wait for the debounce (300ms) + animation (800ms) + buffer.
    print("Waiting for resize debounce and animation...")
    time.sleep(3)

    # 4. Screenshot: Capture the result.
    print("Taking final screenshot...")
    page.screenshot(path="verification/after_resize.png")

    # We can also verify by checking if the transform style of the viewport changed.
    # We expect some transform change.
    viewport_transform = page.locator(".react-flow__viewport").get_attribute("style")
    print(f"Viewport transform after resize: {viewport_transform}")

if __name__ == "__main__":
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        # Start with a large viewport
        page = browser.new_page(viewport={"width": 1920, "height": 1080})
        try:
            test_resize_fit_view(page)
            print("Verification script finished successfully.")
        except Exception as e:
            print(f"Verification script failed: {e}")
            page.screenshot(path="verification/error.png")
            # Dump content to see what's on the page
            with open("verification/error.html", "w") as f:
                f.write(page.content())
        finally:
            browser.close()

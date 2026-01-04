#!/usr/bin/env python3
from playwright.sync_api import sync_playwright
import time

def test_mobile_menu():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        # Mobile viewport (iPhone size)
        context = browser.new_context(viewport={'width': 375, 'height': 667})
        page = context.new_page()

        # Navigate to the app
        page.goto('http://localhost:3000/articles')
        time.sleep(1)

        # Take initial screenshot
        page.screenshot(path='/workspace/before_click.png')
        print("Screenshot saved: before_click.png")

        # Click the hamburger button
        hamburger = page.locator('label[for="mobile-nav-toggle"]').first
        hamburger.click()
        time.sleep(1)

        # Take screenshot after clicking
        page.screenshot(path='/workspace/after_click.png')
        print("Screenshot saved: after_click.png")

        # Check if menu is visible
        menu_panel = page.locator('.mobile-nav-panel')
        is_visible = menu_panel.is_visible()
        print(f"Menu panel visible: {is_visible}")

        # Get computed styles
        menu_transform = page.evaluate('''() => {
            const panel = document.querySelector('.mobile-nav-panel');
            return window.getComputedStyle(panel).transform;
        }''')
        print(f"Menu transform: {menu_transform}")

        menu_zindex = page.evaluate('''() => {
            const panel = document.querySelector('.mobile-nav-panel');
            return window.getComputedStyle(panel).zIndex;
        }''')
        print(f"Menu z-index: {menu_zindex}")

        checkbox_checked = page.evaluate('''() => {
            const checkbox = document.getElementById('mobile-nav-toggle');
            return checkbox.checked;
        }''')
        print(f"Checkbox checked: {checkbox_checked}")

        browser.close()

if __name__ == '__main__':
    test_mobile_menu()

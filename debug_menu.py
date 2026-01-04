#!/usr/bin/env python3
from playwright.sync_api import sync_playwright
import time

def debug_menu():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 375, 'height': 667})
        page = context.new_page()

        page.goto('http://localhost:3000/articles')
        time.sleep(1)

        # Click hamburger
        hamburger = page.locator('label[for="mobile-nav-toggle"]').first
        hamburger.click()
        time.sleep(1)

        # Get all nav links
        links = page.locator('.mobile-nav-panel nav a').all()
        print(f"Number of links found: {len(links)}")
        for i, link in enumerate(links):
            text = link.inner_text()
            visible = link.is_visible()
            box = link.bounding_box()
            print(f"Link {i+1}: '{text}' - Visible: {visible} - Box: {box}")

        # Get panel dimensions
        panel = page.locator('.mobile-nav-panel')
        panel_box = panel.bounding_box()
        print(f"\nPanel bounding box: {panel_box}")

        # Get viewport height
        viewport_height = page.evaluate('window.innerHeight')
        print(f"Viewport height: {viewport_height}")

        # Get computed height of panel
        panel_height = page.evaluate('''() => {
            const panel = document.querySelector('.mobile-nav-panel');
            const styles = window.getComputedStyle(panel);
            return {
                height: styles.height,
                minHeight: styles.minHeight,
                maxHeight: styles.maxHeight
            };
        }''')
        print(f"Panel computed styles: {panel_height}")

        browser.close()

if __name__ == '__main__':
    debug_menu()

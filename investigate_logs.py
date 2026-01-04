#!/usr/bin/env python3
from playwright.sync_api import sync_playwright
import time

def investigate():
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

        # Get the nav element
        nav = page.locator('.mobile-nav-panel nav')
        nav_html = nav.evaluate('el => el.outerHTML')
        print("Nav HTML:")
        print(nav_html)
        print("\n")

        # Check if Logs link exists
        logs_link = page.locator('.mobile-nav-panel nav a:has-text("Logs")')
        logs_exists = logs_link.count()
        print(f"Logs link count: {logs_exists}")

        if logs_exists > 0:
            logs_visible = logs_link.is_visible()
            logs_box = logs_link.bounding_box()
            print(f"Logs visible: {logs_visible}")
            print(f"Logs box: {logs_box}")

        # Get panel scroll height
        scroll_info = page.evaluate('''() => {
            const panel = document.querySelector('.mobile-nav-panel');
            return {
                scrollHeight: panel.scrollHeight,
                clientHeight: panel.clientHeight,
                offsetHeight: panel.offsetHeight
            };
        }''')
        print(f"\nPanel scroll info: {scroll_info}")

        # Take a taller screenshot
        page.screenshot(path='/workspace/full_menu.png', full_page=False)
        print("Full screenshot saved")

        browser.close()

if __name__ == '__main__':
    investigate()

#!/usr/bin/env python3
from playwright.sync_api import sync_playwright
import time

def check_html():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 375, 'height': 667})
        page = context.new_page()

        page.goto('http://localhost:3000/articles')
        time.sleep(1)

        # Get the panel HTML
        panel_html = page.locator('.mobile-nav-panel').evaluate('el => el.outerHTML')
        print("Panel HTML:")
        print(panel_html[:500])
        print("\n")

        # Get panel classes
        panel_classes = page.locator('.mobile-nav-panel').get_attribute('class')
        print(f"Panel classes: {panel_classes}")

        browser.close()

if __name__ == '__main__':
    check_html()

#!/usr/bin/env python3
from playwright.sync_api import sync_playwright
import time

def check_color():
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

        # Get computed styles of all three links
        for link_text in ['Articles', 'Feeds', 'Logs']:
            styles = page.evaluate(f'''() => {{
                const link = Array.from(document.querySelectorAll('.mobile-nav-panel nav a')).find(a => a.textContent.trim() === '{link_text}');
                if (!link) return null;
                const computed = window.getComputedStyle(link);
                const rect = link.getBoundingClientRect();
                return {{
                    color: computed.color,
                    backgroundColor: computed.backgroundColor,
                    display: computed.display,
                    visibility: computed.visibility,
                    opacity: computed.opacity,
                    zIndex: computed.zIndex,
                    position: computed.position,
                    top: rect.top,
                    left: rect.left,
                    width: rect.width,
                    height: rect.height
                }};
            }}''')
            print(f"\n{link_text} link styles:")
            print(styles)

        # Check what element is at the Logs position
        element_at_logs = page.evaluate('''() => {
            const x = 50; // middle of panel
            const y = 210; // where Logs should be
            const el = document.elementFromPoint(x, y);
            if (!el) return 'null';
            return {
                tagName: el.tagName,
                className: el.className,
                id: el.id,
                textContent: el.textContent?.substring(0, 50)
            };
        }''')
        print(f"\nElement at Logs position (50, 210): {element_at_logs}")

        browser.close()

if __name__ == '__main__':
    check_color()

# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "playwright",
# ]
# ///
"""Take screenshots of FluxFeed articles page for documentation."""

import asyncio
from playwright.async_api import async_playwright

BASE_URL = "http://localhost:3333"
OUTPUT_DIR = "./docs"

VIEWPORT_CONFIGS = [
    {"name": "desktop", "width": 1920, "height": 1080},
    {"name": "mobile", "width": 390, "height": 844},  # iPhone 14 Pro
]

THEMES = ["light", "dark"]


async def take_screenshots():
    async with async_playwright() as p:
        browser = await p.chromium.launch()

        for viewport in VIEWPORT_CONFIGS:
            for theme in THEMES:
                context = await browser.new_context(
                    viewport={"width": viewport["width"], "height": viewport["height"]},
                    color_scheme=theme,
                )
                page = await context.new_page()

                # Navigate to articles page
                await page.goto(f"{BASE_URL}/articles")

                # Wait for content to load
                await page.wait_for_load_state("networkidle")
                await asyncio.sleep(0.5)  # Extra time for any animations

                # Take screenshot
                filename = f"{OUTPUT_DIR}/articles-{viewport['name']}-{theme}.png"
                await page.screenshot(path=filename, full_page=False)
                print(f"Saved: {filename}")

                await context.close()

        await browser.close()


if __name__ == "__main__":
    asyncio.run(take_screenshots())

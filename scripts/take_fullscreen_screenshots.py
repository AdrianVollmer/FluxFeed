# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "playwright",
# ]
# ///
"""
Take screenshots of FluxFeed fullscreen view for documentation.

This script:
1. Starts the server with a temporary database
2. Adds RSS feeds and fetches articles
3. Takes light and dark theme screenshots in the same session
4. Generates a collage combining both themes
"""

import asyncio
import os
import shutil
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

from playwright.async_api import async_playwright

BASE_URL = "http://localhost:3000"
WORKSPACE = Path(__file__).parent.parent
OUTPUT_DIR = WORKSPACE / "docs"

FEEDS = [
    "https://hnrss.org/frontpage",
    "https://lobste.rs/rss",
    "https://www.reddit.com/r/programming/.rss",
    "https://blog.rust-lang.org/feed.xml",
]


class ServerManager:
    """Manage the FluxFeed server process."""

    def __init__(self, db_path: str):
        self.db_path = db_path
        self.process = None

    def start(self):
        """Start the server."""
        env = os.environ.copy()
        env["DATABASE_URL"] = f"sqlite://{self.db_path}"
        env["SQLX_OFFLINE"] = "true"

        binary = WORKSPACE / "target" / "release" / "fluxfeed"
        if not binary.exists():
            print("Building release binary...")
            subprocess.run(
                ["cargo", "build", "--release"],
                cwd=WORKSPACE,
                env=env,
                check=True,
            )

        self.process = subprocess.Popen(
            [str(binary)],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        print(f"Started server (PID: {self.process.pid})")

    def stop(self):
        """Stop the server."""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            print("Server stopped")

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *args):
        self.stop()


async def wait_for_server(max_retries: int = 30):
    """Wait for the server to be ready."""
    import urllib.request
    import urllib.error

    for i in range(max_retries):
        try:
            with urllib.request.urlopen(f"{BASE_URL}/health", timeout=1) as resp:
                if resp.status == 200:
                    print("Server is ready")
                    return True
        except (urllib.error.URLError, OSError):
            pass
        await asyncio.sleep(0.5)

    raise RuntimeError("Server did not start in time")


async def add_feeds_and_fetch(page):
    """Add RSS feeds via the web interface."""
    for feed_url in FEEDS:
        await page.goto(f"{BASE_URL}/feeds")
        await page.wait_for_load_state("networkidle")
        await asyncio.sleep(0.5)

        # Click "Add Feed" and wait for HTMX response
        async with page.expect_response("**/feeds/new"):
            await page.locator('button:has-text("Add Feed")').first.click()

        await asyncio.sleep(0.3)

        # Wait for modal input
        try:
            await page.wait_for_selector('input[name="url"]', state="visible", timeout=5000)
        except Exception as e:
            print(f"Modal did not appear for {feed_url}: {e}")
            continue

        await page.fill('input[name="url"]', feed_url)

        # Submit and wait for response
        async with page.expect_response("**/feeds"):
            await page.locator('button[type="submit"]:has-text("Add Feed")').click()

        await asyncio.sleep(2)
        print(f"Added feed: {feed_url}")

    # Fetch each feed
    await page.goto(f"{BASE_URL}/feeds")
    await page.wait_for_load_state("networkidle")

    feed_links = await page.locator('a[href^="/feeds/"]').all()
    feed_ids = set()
    for link in feed_links:
        href = await link.get_attribute("href")
        if href and href.startswith("/feeds/") and href.count("/") == 2:
            feed_id = href.split("/")[-1]
            if feed_id.isdigit():
                feed_ids.add(feed_id)

    for feed_id in sorted(feed_ids, key=int):
        await page.goto(f"{BASE_URL}/feeds/{feed_id}")
        await page.wait_for_load_state("networkidle")

        fetch_btn = page.locator('button:has-text("Fetch now")').first
        if await fetch_btn.is_visible():
            await fetch_btn.click()
            await asyncio.sleep(2)
            print(f"Fetched feed {feed_id}")


async def take_screenshots(page, context):
    """Take screenshots in both themes using the same page state."""
    # Set fullscreen view cookie
    await context.add_cookies([
        {"name": "articleView", "value": "fullscreen", "url": BASE_URL}
    ])

    # Navigate to articles in light mode first
    await page.goto(f"{BASE_URL}/articles")
    await page.wait_for_load_state("networkidle")
    await asyncio.sleep(0.5)

    # Click on first article to show content
    first_article = page.locator('.fullscreen-article-row').first
    if await first_article.is_visible():
        await first_article.click()
        await asyncio.sleep(1)

    # Take light screenshot
    light_file = OUTPUT_DIR / "articles-desktop-light-fullscreen.png"
    await page.screenshot(path=str(light_file), full_page=False)
    print(f"Saved: {light_file}")

    # Switch to dark mode using emulateMedia
    await page.emulate_media(color_scheme="dark")
    await asyncio.sleep(0.3)

    # Take dark screenshot (same page state, just different theme)
    dark_file = OUTPUT_DIR / "articles-desktop-dark-fullscreen.png"
    await page.screenshot(path=str(dark_file), full_page=False)
    print(f"Saved: {dark_file}")

    return light_file, dark_file


def generate_collage(light_file: Path, dark_file: Path):
    """Generate a collage combining light and dark screenshots."""
    output_file = OUTPUT_DIR / "articles-desktop-fullscreen-collage.png"

    # Check if ImageMagick is available
    if not shutil.which("convert"):
        print("ImageMagick not found, skipping collage generation")
        return

    w, h = 1920, 1080

    # Use a temp directory for intermediate files
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Create diagonal mask
        subprocess.run([
            "convert", "-size", f"{w}x{h}", "xc:black",
            "-fill", "white", "-draw", f"polygon 0,0 {w},0 {w},{h}",
            str(tmpdir / "mask.png")
        ], check=True)

        # Apply mask to light image
        subprocess.run([
            "convert", str(light_file), str(tmpdir / "mask.png"),
            "-alpha", "off", "-compose", "CopyOpacity", "-composite",
            str(tmpdir / "light_masked.png")
        ], check=True)

        # Composite dark base with masked light overlay
        subprocess.run([
            "convert", str(dark_file), str(tmpdir / "light_masked.png"),
            "-compose", "Over", "-composite",
            str(tmpdir / "out.png")
        ], check=True)

        # Create glowing diagonal line
        subprocess.run([
            "convert", "-size", f"{w}x{h}", "xc:transparent",
            "-stroke", "rgba(255,255,255,0.95)", "-strokewidth", "3",
            "-draw", f"line 0,0 {w},{h}",
            str(tmpdir / "line.png")
        ], check=True)

        subprocess.run([
            "convert", str(tmpdir / "line.png"),
            "-blur", "0x8", "-level", "0%,100%,0.4",
            str(tmpdir / "glow.png")
        ], check=True)

        subprocess.run([
            "convert", str(tmpdir / "glow.png"), str(tmpdir / "line.png"),
            "-composite", str(tmpdir / "line_final.png")
        ], check=True)

        subprocess.run([
            "convert", str(tmpdir / "out.png"), str(tmpdir / "line_final.png"),
            "-composite", str(output_file)
        ], check=True)

    print(f"Generated collage: {output_file}")


async def main():
    # Create temporary database
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as tmp:
        db_path = tmp.name

    try:
        # Start server
        with ServerManager(db_path) as server:
            await wait_for_server()

            async with async_playwright() as p:
                browser = await p.chromium.launch()

                # Create context with light theme initially
                context = await browser.new_context(
                    viewport={"width": 1920, "height": 1080},
                    color_scheme="light",
                )
                page = await context.new_page()

                # Add feeds and fetch articles
                await add_feeds_and_fetch(page)

                # Take screenshots (both themes, same session)
                light_file, dark_file = await take_screenshots(page, context)

                await context.close()
                await browser.close()

            # Generate collage
            generate_collage(light_file, dark_file)

    finally:
        # Clean up temp database
        try:
            os.unlink(db_path)
        except OSError:
            pass

    print("\nDone! Screenshots saved to docs/")


if __name__ == "__main__":
    asyncio.run(main())

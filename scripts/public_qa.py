#!/usr/bin/env python3
"""
Public route QA for OpenCorde.

Checks public routes across common viewports for:
- console errors
- page errors
- failed requests
- horizontal overflow

Environment:
  OC_BASE=https://opencorde.com
  OC_PUBLIC_QA_OUT=reports/raw/public-qa.json
  PLAYWRIGHT_CHROME=/usr/bin/google-chrome
"""

import argparse
import asyncio
import json
import os
from pathlib import Path

from playwright.async_api import async_playwright


DEFAULT_ROUTES = [
    "/",
    "/login",
    "/register",
    "/servers",
    "/invite/invalid-code-123",
]

DEFAULT_VIEWPORTS = [
    ("mobile", 390, 844),
    ("tablet", 768, 1024),
    ("desktop", 1280, 800),
]


async def run_check(base_url: str, routes: list[str], viewports: list[tuple[str, int, int]]) -> list[dict]:
    results: list[dict] = []
    chrome = os.environ.get("PLAYWRIGHT_CHROME", "/usr/bin/google-chrome")

    async with async_playwright() as pw:
        browser = await pw.chromium.launch(
            executable_path=chrome,
            args=["--no-sandbox", "--disable-dev-shm-usage", "--headless=new"],
        )

        for viewport_name, width, height in viewports:
            context = await browser.new_context(viewport={"width": width, "height": height})

            for route in routes:
                page = await context.new_page()
                console_errors: list[dict] = []
                page_errors: list[str] = []
                failed_requests: list[dict] = []

                page.on(
                    "console",
                    lambda msg, bucket=console_errors: bucket.append(
                        {"type": msg.type, "text": msg.text}
                    )
                    if msg.type == "error"
                    else None,
                )
                page.on("pageerror", lambda err, bucket=page_errors: bucket.append(str(err)))
                page.on(
                    "requestfailed",
                    lambda req, bucket=failed_requests: bucket.append(
                        {"url": req.url, "failure": req.failure}
                    ),
                )

                url = f"{base_url}{route}"
                status = None
                final_url = url
                title = ""
                h1 = ""
                horizontal_overflow = False

                try:
                    response = await page.goto(url, wait_until="networkidle", timeout=25000)
                    status = response.status if response else None
                    final_url = page.url
                    title = await page.title()
                    h1 = await page.locator("h1").first.text_content(timeout=1000) or ""
                    horizontal_overflow = await page.evaluate(
                        "document.documentElement.scrollWidth > document.documentElement.clientWidth"
                    )
                except Exception as exc:
                    page_errors.append(str(exc))

                results.append(
                    {
                        "browser": "chromium",
                        "viewport": viewport_name,
                        "width": width,
                        "height": height,
                        "route": route,
                        "status": status,
                        "finalUrl": final_url,
                        "title": title,
                        "h1": h1.strip(),
                        "horizontalOverflow": horizontal_overflow,
                        "console": console_errors,
                        "pageErrors": page_errors,
                        "failedRequests": failed_requests,
                        "ok": (
                            not horizontal_overflow
                            and not console_errors
                            and not page_errors
                            and not failed_requests
                        ),
                    }
                )

                await page.close()

            await context.close()

        await browser.close()

    return results


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=os.environ.get("OC_BASE", "https://opencorde.com"))
    parser.add_argument(
        "--out",
        default=os.environ.get("OC_PUBLIC_QA_OUT", "reports/raw/public-qa.json"),
    )
    parser.add_argument("--fail-on-issues", action="store_true")
    return parser.parse_args()


async def main() -> int:
    args = parse_args()
    base_url = args.base.rstrip("/")
    results = await run_check(base_url, DEFAULT_ROUTES, DEFAULT_VIEWPORTS)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")

    failed = [item for item in results if not item["ok"]]
    for item in results:
        status = "PASS" if item["ok"] else "FAIL"
        print(
            f"[{status}] {item['viewport']} {item['route']} "
            f"overflow={item['horizontalOverflow']} "
            f"console={len(item['console'])} "
            f"pageErrors={len(item['pageErrors'])} "
            f"failedRequests={len(item['failedRequests'])}"
        )

    print(f"Wrote {out_path}")
    if failed and args.fail_on_issues:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))

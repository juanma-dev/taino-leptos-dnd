#!/usr/bin/env python3
"""E2E smoke test: real cross-zone drags in the multi-zone examples.

Drives a headless Chrome via the WebDriver protocol (no selenium needed):
builds an example with trunk, serves its dist/, dispatches a real
pointerdown/move/up drag from a card to another zone, and asserts the
card actually moved in the DOM. This is the automated version of the
manual "drag a card from Zone A to Zone B" check.

Prerequisites (see docs/CONTRIBUTING.md):
  - trunk on PATH (or TRUNK env var)
  - Chrome for Testing + chromedriver, e.g. in ~/.local/cft/
    (override with CHROME / CHROMEDRIVER env vars)

Usage:
  scripts/e2e-smoke.py                    # both multi-zone examples
  scripts/e2e-smoke.py multi-zone-dioxus  # just one
  E2E_NO_BUILD=1 scripts/e2e-smoke.py     # reuse existing dist/
"""
import base64
import json
import os
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from threading import Thread

REPO = Path(__file__).resolve().parent.parent
CFT = Path.home() / ".local" / "cft"
CHROME = os.environ.get(
    "CHROME", str(CFT / "chrome-headless-shell-linux64" / "chrome-headless-shell"))
CHROMEDRIVER = os.environ.get(
    "CHROMEDRIVER", str(CFT / "chromedriver-linux64" / "chromedriver"))
TRUNK = os.environ.get("TRUNK", shutil.which("trunk") or str(CFT / "trunk"))

# (card to drag, destination zone) — one vertical->vertical, one bar->bar.
DRAGS = [("A · ship feature", "Zone B"), ("C1", "Bar D")]

ZONES_JS = """
return Array.from(document.querySelectorAll('section')).map(s => ({
  zone: s.getAttribute('aria-label'),
  cards: Array.from(s.querySelectorAll('.card')).map(c => c.getAttribute('aria-label'))
}));
"""

DRAG_JS = """
const [fromLabel, toZone] = arguments;
const card = document.querySelector(`.card[aria-label="${fromLabel}"]`);
if (!card) return 'NO CARD';
const target = Array.from(document.querySelectorAll('section'))
  .find(s => s.getAttribute('aria-label') === toZone);
if (!target) return 'NO TARGET ZONE';
// Aim at the zone's tail droppable ("drop at end" slot).
const tail = target.querySelector('[aria-hidden="true"]') || target;
const c = card.getBoundingClientRect();
const t = tail.getBoundingClientRect();
const from = { x: c.x + c.width / 2, y: c.y + c.height / 2 };
const to = { x: t.x + t.width / 2, y: t.y + t.height / 2 };
function fire(kind, x, y) {
  card.dispatchEvent(new PointerEvent(kind, {
    bubbles: true, cancelable: true, clientX: x, clientY: y,
    button: 0, buttons: 1, pointerId: 1, pointerType: 'mouse', isPrimary: true,
  }));
}
fire('pointerdown', from.x, from.y);
for (let i = 1; i <= 8; i++) {
  fire('pointermove', from.x + (to.x - from.x) * i / 8,
                      from.y + (to.y - from.y) * i / 8);
}
fire('pointerup', to.x, to.y);
return 'OK';
"""


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wd(method, path, body=None, port=9515):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=data,
                                 method=method,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.load(r)["value"]


def run_example(name, driver_port):
    example = REPO / "examples" / name
    if not os.environ.get("E2E_NO_BUILD"):
        print(f"[{name}] trunk build ...")
        subprocess.run([TRUNK, "build"], cwd=example, check=True,
                       capture_output=True)

    http_port = free_port()
    os.chdir(example / "dist")
    server = ThreadingHTTPServer(("127.0.0.1", http_port), SimpleHTTPRequestHandler)
    Thread(target=server.serve_forever, daemon=True).start()

    sid = wd("POST", "/session", {"capabilities": {"alwaysMatch": {
        "browserName": "chrome",
        "goog:chromeOptions": {
            "binary": CHROME,
            "args": ["--no-sandbox", "--disable-dev-shm-usage",
                     "--window-size=1280,900"],
        },
    }}}, port=driver_port)["sessionId"]
    failed = 0
    try:
        wd("POST", f"/session/{sid}/url",
           {"url": f"http://127.0.0.1:{http_port}/"}, port=driver_port)
        ex = lambda script, args=None: wd(
            "POST", f"/session/{sid}/execute/sync",
            {"script": script, "args": args or []}, port=driver_port)
        for _ in range(50):
            if ex("return document.querySelectorAll('.card').length") > 0:
                break
            time.sleep(0.2)
        else:
            print(f"[{name}] FAIL: app never rendered any .card")
            return 1

        for from_label, to_zone in DRAGS:
            dispatch = ex(DRAG_JS, [from_label, to_zone])
            time.sleep(0.8)  # drop-settle + FLIP transitions
            zones = ex(ZONES_JS)
            dest = next((z for z in zones if z["zone"] == to_zone), None)
            ok = dispatch == "OK" and dest and from_label in dest["cards"]
            print(f"[{name}] '{from_label}' -> {to_zone}: "
                  + ("MOVED ok" if ok else f"FAILED ({dispatch})"))
            if not ok:
                failed += 1
                png = wd("GET", f"/session/{sid}/screenshot", port=driver_port)
                shot = f"/tmp/e2e-{name}-fail.png"
                Path(shot).write_bytes(base64.b64decode(png))
                print(f"[{name}] screenshot: {shot}")
    finally:
        wd("DELETE", f"/session/{sid}", port=driver_port)
        server.shutdown()
    return failed


def main():
    examples = sys.argv[1:] or ["multi-zone", "multi-zone-dioxus"]
    driver_port = free_port()
    driver = subprocess.Popen([CHROMEDRIVER, f"--port={driver_port}"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.0)
    try:
        failed = sum(run_example(name, driver_port) for name in examples)
    finally:
        driver.terminate()
    print("ALL GREEN" if failed == 0 else f"{failed} DRAG(S) FAILED")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()

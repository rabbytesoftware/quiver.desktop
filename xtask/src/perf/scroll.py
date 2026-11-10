"""Scroll smoothness over the bridge: real wheel events through xdotool, frames counted by the app (app.ping)."""
import subprocess
import tempfile
import time
import shutil
import os

import environment as envmod
import launcher
import x11
from bridge import Bridge

SCENARIOS = {
    # scenario: (QUIVER_DEMO_ROUTE or None, node that proves the screen is up, node to scroll over)
    "sidebar": (None, "sidebar.list", "sidebar.list"),
    "library": ("library", "screen.library", "screen.library"),
    "readme": ("arrow:github.com/rabbyte/minecraft", "arrow.readme", "screen.arrow"),
}


def frames(bridge):
    return bridge.call("app.ping")["frame"]


def wheel(display, x, y, count, delay_ms):
    env = {"DISPLAY": display, "PATH": "/usr/bin:/bin"}
    subprocess.run(["xdotool", "mousemove", str(x), str(y), "click", "--repeat", str(count), "--delay", str(delay_ms), "5"], env=env, check=True)


def burst(bridge, display, x, y, clicks, delay_ms):
    """Fires `clicks` wheel notches and watches the frame counter until it stays still for 400 ms."""
    start_frame, start = frames(bridge), time.monotonic()
    wheel(display, x, y, clicks, delay_ms)
    sent = time.monotonic()
    last, last_change = frames(bridge), time.monotonic()
    while time.monotonic() - last_change < 0.4:
        f = frames(bridge)
        if f != last:
            last, last_change = f, time.monotonic()
        time.sleep(0.002)
    painted = last - start_frame
    elapsed = last_change - start
    return {
        "wheel_clicks": clicks, "frames": painted, "elapsed_ms": round(elapsed * 1000, 1),
        "input_ms": round((sent - start) * 1000, 1),
        "ms_per_frame": round(elapsed * 1000 / painted, 2) if painted else None,
    }


def latency(bridge, display, x, y, steps):
    """One notch at a time: ms from the notch being sent to the next frame."""
    samples = []
    for _ in range(steps):
        f0 = frames(bridge)
        t = time.monotonic()
        wheel(display, x, y, 1, 0)
        t_sent = time.monotonic()
        while frames(bridge) == f0 and time.monotonic() - t < 5:
            time.sleep(0.001)
        samples.append((time.monotonic() - t_sent) * 1000)
    samples.sort()
    return {"steps": steps, "median_ms": round(samples[len(samples) // 2], 1), "p90_ms": round(samples[int(len(samples) * 0.9)], 1), "max_ms": round(samples[-1], 1)}


def run_scroll(args):
    route, ready_node, over = SCENARIOS[args.scenario]
    home = tempfile.mkdtemp(prefix="perf-home-")
    os.makedirs(os.path.join(home, ".quiver"))
    extra = {"QUIVER_DEMO_ARROWS": str(args.count)}
    if route:
        extra["QUIVER_DEMO_ROUTE"] = route
    if args.scenario == "readme":
        extra["QUIVER_DEMO_README_KB"] = str(args.readme_kb)
    app = launcher.Launched("native-bridge", extra, ["--demo"], home)
    app.port_event.wait(30)
    try:
        bridge = Bridge(app.port)
        t0 = time.monotonic()
        bridge.signal("first-frame")
        node = bridge.node(ready_node)
        ready_ms = round((time.monotonic() - app.t0) * 1000, 1)
        target = bridge.call("ui.find", {"id": over}) or node
        window = app.watcher.wait_window(30)
        gx, gy, _, _ = x11.geometry(envmod.DISPLAY, window)
        x, y = int(gx + target["x"] + target["w"] / 2), int(gy + target["y"] + min(target["h"], 600) / 2)
        idle0, ti = frames(bridge), time.monotonic()
        time.sleep(3)
        result = {
            "scenario": args.scenario, "arrows": args.count, "readme_kb": args.readme_kb if args.scenario == "readme" else None,
            "ready_ms": ready_ms, "idle_frames_per_s_before": round((frames(bridge) - idle0) / (time.monotonic() - ti), 2),
            "single_notch_latency": latency(bridge, envmod.DISPLAY, x, y, args.steps),
            "burst_fast": burst(bridge, envmod.DISPLAY, x, y, args.steps * 3, 8),
            "burst_slow": burst(bridge, envmod.DISPLAY, x, y, args.steps, 40),
            "note": "software rendering (lavapipe) in a shared container: relative numbers only, see docs/performance.md",
        }
        return result
    finally:
        app.stop()
        shutil.rmtree(home, ignore_errors=True)

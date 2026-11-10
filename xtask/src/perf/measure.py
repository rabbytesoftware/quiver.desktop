#!/usr/bin/env python3
"""Performance measurements of the Quiver Desktop apps inside the perf box (Xvfb + openbox + software rendering).

  measure.py startup --variant native-release|native-bridge|tauri [--runs 5] [--idle 30] [--demo] [--out file.json]
  measure.py scroll  --scenario sidebar|library|readme --count N [--steps 60] [--out file.json]

Nothing here is a test: it is a measurement script, so it sleeps for fixed windows on purpose (settle time, the idle CPU window).
All timestamps are time.monotonic() deltas from just before the app is spawned.
"""
import argparse
import json
import os
import shutil
import sys
import tempfile
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import environment as envmod  # noqa: E402
import procstat  # noqa: E402
import stats  # noqa: E402
import x11  # noqa: E402
from bridge import Bridge  # noqa: E402
from launcher import CORE_COMM, VARIANTS, Launched, watch_core_socket  # noqa: E402

SETTLE_S = 10


def one_startup(variant, idle_s, demo, extra_env):
    home = tempfile.mkdtemp(prefix="perf-home-")
    os.makedirs(os.path.join(home, ".quiver"))
    envmod.kill_leftovers()
    app = Launched(variant, extra_env, ["--demo"] if demo else [], home)
    result = {"variant": variant}
    sock, stop = {}, threading.Event()
    if not demo:
        threading.Thread(target=watch_core_socket, args=(home, sock, stop), daemon=True).start()
    try:
        window = app.watcher.wait_window(60)
        result["window_mapped_ms"] = app.ms(app.watcher.mapped_at)
        painted = x11.wait_painted(envmod.DISPLAY, window, 60) if window else None
        result["first_painted_ms"] = app.ms(painted)
        bridge = None
        if VARIANTS[variant]["bridge"] and app.port_event.wait(30):
            bridge = Bridge(app.port)
            result["bridge_connected_ms"] = app.ms(time.monotonic())
            for name, key in (("first-frame", "first_frame_signal_ms"), ("core.ready", "core_ready_signal_ms")):
                if demo and name == "core.ready":
                    continue
                before = time.monotonic()
                bridge.signal(name)
                after = time.monotonic()
                result[key] = app.ms(after)
                # A latched signal answers at once: then the value is only an upper bound.
                result[key + "_waited_ms"] = round((after - before) * 1000, 1)
            bridge.node("sidebar.list")
            result["home_ms"] = app.ms(time.monotonic())
        elif not demo:
            deadline = time.monotonic() + 60
            while "at" not in sock and time.monotonic() < deadline:
                time.sleep(0.01)
            result["core_socket_ms"] = app.ms(sock.get("at"))
            if sock.get("at") and painted:
                result["home_ms"] = app.ms(max(sock["at"], painted))
        if "core_socket_ms" not in result and sock.get("at"):
            result["core_socket_ms"] = app.ms(sock["at"])
        time.sleep(SETTLE_S)

        pids = app.tree()
        core = app.core_pids()
        app_only = [p for p in pids if p not in core]
        result["processes"] = {str(p): procstat.comm(p) for p in app_only}
        mem = procstat.memory(app_only)
        result["rss_mb"] = round(mem["rss_kb"] / 1024, 1)
        result["pss_mb"] = round(mem["pss_kb"] / 1024, 1)
        result["main_process_rss_mb"] = round(procstat.rss_kb(app.proc.pid) / 1024, 1)
        core_mem = procstat.memory(core)
        result["core_rss_mb"] = round(core_mem["rss_kb"] / 1024, 1)

        frame0 = bridge.frame() if bridge else None
        before, core_before = procstat.snapshot(app_only), procstat.snapshot(core)
        t1 = time.monotonic()
        time.sleep(idle_s)
        elapsed = time.monotonic() - t1
        after, core_after = procstat.snapshot(app_only), procstat.snapshot(core)
        idle = procstat.delta(before, after, elapsed)
        result["idle_cpu_percent"] = idle["cpu_percent"]
        result["idle_wakeups_per_s"] = idle["wakeups_per_s"]
        result["idle_top_threads"] = idle["top_threads"]
        core_idle = procstat.delta(core_before, core_after, elapsed)
        result["core_idle_cpu_percent"] = core_idle["cpu_percent"]
        result["core_idle_wakeups_per_s"] = core_idle["wakeups_per_s"]
        if bridge:
            result["idle_frames_per_s"] = round((bridge.frame() - frame0) / elapsed, 2)
        result["idle_window_s"] = round(elapsed, 1)
    finally:
        stop.set()
        app.stop()
        shutil.rmtree(home, ignore_errors=True)
    return result


def sizes(variant):
    d = VARIANTS[variant]["dir"]
    out = {}
    for name in ("quiverdesktop", "quiver-arrow-window", "quiver"):
        path = os.path.join(d, name)
        if os.path.exists(path):
            out[name + "_mb"] = round(os.path.getsize(path) / 1048576, 2)
    return out


def startup(args):
    runs = []
    for i in range(args.runs):
        r = one_startup(args.variant, args.idle, args.demo, dict(kv.split("=", 1) for kv in args.env))
        print(f"run {i + 1}/{args.runs}: {json.dumps({k: v for k, v in r.items() if k not in ('processes', 'idle_top_threads')})}", flush=True)
        runs.append(r)
    doc = {
        "variant": args.variant, "demo": args.demo, "runs": runs, "summary": stats.summarise_runs(runs),
        "binary_sizes": sizes(args.variant), "cpus": os.cpu_count(), "loadavg_at_end": os.getloadavg(),
        "env_note": "Xvfb + openbox, software rendering (lavapipe/llvmpipe), shared container: see docs/performance.md",
    }
    emit(doc, args.out)


def emit(doc, out):
    text = json.dumps(doc, indent=2)
    print(text)
    if out:
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with open(out, "w") as f:
            f.write(text + "\n")


def scroll(args):
    from scroll import run_scroll

    emit(run_scroll(args), args.out)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("startup")
    s.add_argument("--variant", choices=VARIANTS, required=True)
    s.add_argument("--runs", type=int, default=5)
    s.add_argument("--idle", type=int, default=30)
    s.add_argument("--demo", action="store_true")
    s.add_argument("--env", action="append", default=[])
    s.add_argument("--out")
    c = sub.add_parser("scroll")
    c.add_argument("--scenario", choices=("sidebar", "library", "readme"), required=True)
    c.add_argument("--count", type=int, default=5000)
    c.add_argument("--steps", type=int, default=60)
    c.add_argument("--readme-kb", type=int, default=500)
    c.add_argument("--out")
    args = ap.parse_args()
    procs = envmod.start_desktop()
    try:
        {"startup": startup, "scroll": scroll}[args.cmd](args)
    finally:
        for p in procs:
            p.kill()


if __name__ == "__main__":
    main()

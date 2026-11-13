"""Spawns an app variant under the watchers: X11 map events, the bridge port, the process tree."""
import os
import socket
import subprocess
import threading
import time

import environment as envmod
import procstat
import x11

VARIANTS = {
    "native-release": {"dir": "/perf/native/release", "bridge": False},
    "native-bridge": {"dir": "/perf/native/bridge", "bridge": True},
    "tauri": {"dir": "/perf/tauri", "bridge": False},
    # Builds from before the idle-repaint fix (Query::run), kept on the box that measured the fix for the before/after.
    "native-release-before": {"dir": "/perf/native-before/release", "bridge": False},
    "native-bridge-before": {"dir": "/perf/native-before/bridge", "bridge": True},
}
CORE_COMM = "quiver"


class Launched:
    def __init__(self, variant, extra_env, args, home):
        self.variant, self.home = variant, home
        spec = VARIANTS[variant]
        env = envmod.app_env(home, extra_env)
        if spec["bridge"]:
            env["QUIVER_E2E_PORT"] = "0"
        self.watcher = x11.WindowWatcher(envmod.DISPLAY)
        self.port = None
        self.port_event = threading.Event()
        self.t0 = time.monotonic()
        self.proc = subprocess.Popen(
            [os.path.join(spec["dir"], "quiverdesktop"), *args], env=env, cwd=home,
            stdout=subprocess.PIPE, stderr=open(os.path.join(home, "app.stderr"), "w"), text=True,
        )
        self.watcher.pid = self.proc.pid
        threading.Thread(target=self._stdout, daemon=True).start()

    def _stdout(self):
        for line in self.proc.stdout:
            if line.startswith("QUIVER_BRIDGE_PORT="):
                self.port = int(line.split("=")[1])
                self.port_event.set()

    def ms(self, stamp):
        return None if stamp is None else round((stamp - self.t0) * 1000, 1)

    def tree(self):
        return [p for p in procstat.descendants(self.proc.pid)]

    def core_pids(self):
        return procstat.pids_named(CORE_COMM)

    def stop(self):
        self.watcher.close()
        envmod.kill_leftovers()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def watch_core_socket(home, out, stop):
    """Stamps the first moment the daemon's unix socket accepts a connection (10 ms probe; a socket has no event)."""
    path = os.path.join(home, ".quiver", "quiver.sock")
    while not stop.is_set():
        if os.path.exists(path):
            s = socket.socket(socket.AF_UNIX)
            try:
                s.connect(path)
                out["at"] = time.monotonic()
                return
            except OSError:
                pass
            finally:
                s.close()
        time.sleep(0.01)



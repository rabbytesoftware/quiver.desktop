"""First-window timing from X11 events (xev on the root window, no polling) and a pixel check for 'painted'."""
import subprocess
import threading
import time

from PIL import ImageGrab


class WindowWatcher:
    """Starts `xev -root` BEFORE the app, stamps every MapNotify with the monotonic clock, and resolves the app's
    window with `xdotool search --pid` on each one until it exists."""

    def __init__(self, display):
        self.display = display
        self.pid = None
        self.window = None
        self.mapped_at = None
        self._cv = threading.Condition()
        self.proc = subprocess.Popen(
            ["xev", "-display", display, "-root", "-event", "substructure"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1,
        )
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        for line in self.proc.stdout:
            if not line.startswith("MapNotify") or self.pid is None or self.window:
                continue
            stamp = time.monotonic()
            out = subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--pid", str(self.pid)],
                capture_output=True, text=True, env={"DISPLAY": self.display, "PATH": "/usr/bin:/bin"},
            ).stdout.split()
            if out:
                with self._cv:
                    self.window, self.mapped_at = out[0], stamp
                    self._cv.notify_all()

    def wait_window(self, timeout):
        with self._cv:
            self._cv.wait_for(lambda: self.window, timeout)
        return self.window

    def close(self):
        self.proc.kill()


def geometry(display, window):
    info = subprocess.run(["xwininfo", "-display", display, "-id", window], capture_output=True, text=True).stdout
    vals = {}
    for line in info.splitlines():
        line = line.strip()
        for key in ("Absolute upper-left X", "Absolute upper-left Y", "Width", "Height"):
            if line.startswith(key + ":"):
                vals[key] = int(line.split(":")[1])
    return vals["Absolute upper-left X"], vals["Absolute upper-left Y"], vals["Width"], vals["Height"]


def painted(display, window):
    """True once the window's client area holds more than a flat fill (text, icons, tiles)."""
    x, y, w, h = geometry(display, window)
    img = ImageGrab.grab(bbox=(x, y, x + w, y + h), xdisplay=display).convert("RGB").resize((120, 80))
    colors = img.getcolors(maxcolors=10000)
    return colors is None or len(colors) >= 12


def wait_painted(display, window, timeout, period=0.02):
    """Polls (20 ms) the pixels; returns the monotonic time of the first painted grab, or None."""
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        stamp = time.monotonic()
        try:
            if painted(display, window):
                return stamp
        except Exception:
            pass
        time.sleep(period)
    return None

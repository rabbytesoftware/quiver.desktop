"""The virtual desktop the apps run in: Xvfb + openbox + software Vulkan, and cleanup between runs."""
import os
import subprocess
import time

import procstat

DISPLAY = os.environ.get("PERF_DISPLAY", ":77")
SCREEN = "1600x1000x24"


def app_env(home, extra=None):
    env = {
        "PATH": "/usr/local/cargo/bin:/usr/bin:/bin:/usr/local/bin",
        "HOME": home,
        "QUIVER_HOME": os.path.join(home, ".quiver"),
        "DISPLAY": DISPLAY,
        # lavapipe (software Vulkan) for GPUI; the same three switches the old e2e box exported for WebKitGTK.
        "ZED_ALLOW_EMULATED_GPU": "1",
        "VK_ICD_FILENAMES": "/usr/share/vulkan/icd.d/lvp_icd.json",
        "WEBKIT_DISABLE_COMPOSITING_MODE": "1",
        "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
        "WEBKIT_FORCE_SANDBOX": "0",
    }
    env.update(extra or {})
    return env


def start_desktop():
    xvfb = subprocess.Popen(["Xvfb", DISPLAY, "-screen", "0", SCREEN, "-nolisten", "tcp"], stderr=subprocess.DEVNULL)
    for _ in range(100):
        if subprocess.run(["xdpyinfo", "-display", DISPLAY], capture_output=True).returncode == 0:
            break
        time.sleep(0.1)
    wm = subprocess.Popen(["openbox"], env={"DISPLAY": DISPLAY, "PATH": "/usr/bin:/bin"}, stderr=subprocess.DEVNULL)
    for _ in range(100):
        out = subprocess.run(["xprop", "-display", DISPLAY, "-root", "_NET_SUPPORTING_WM_CHECK"], capture_output=True, text=True).stdout
        if "window id" in out:
            break
        time.sleep(0.1)
    return [wm, xvfb]


def kill_leftovers():
    for name in ("quiverdesktop", "quiver", "quiver-arrow-window", "WebKitWebProces", "WebKitNetworkPr"):
        for pid in procstat.pids_named(name):
            try:
                os.kill(pid, 9)
            except OSError:
                pass

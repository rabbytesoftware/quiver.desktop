# Runs inside the native dev container (scripts/box.sh bash scripts/arrow-window-spike.sh): builds the arrow-app
# helper, starts Xvfb + openbox and has docker/native/arrow-window-spike.py serve it a page and screenshot it.
set -e
cd /work
cargo build -p quiver-app --bin quiver-arrow-window 2>&1 | tail -3
export DISPLAY=:1 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 GDK_BACKEND=x11 NO_AT_BRIDGE=1
Xvfb :1 -screen 0 1280x800x24 -nolisten tcp >/dev/null 2>&1 &
timeout 30 bash -c 'until xdpyinfo -display :1 >/dev/null 2>&1; do :; done'
openbox >/dev/null 2>&1 &
timeout 30 bash -c 'until xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q "window id"; do :; done'
python3 docker/native/arrow-window-spike.py target/debug/quiver-arrow-window docker/native/evidence/arrow-window-spike.png

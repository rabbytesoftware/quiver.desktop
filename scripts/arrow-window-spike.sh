set -e
cd /work
cargo build -p quiver-app --bin quiver-arrow-window 2>&1 | tail -3
export DISPLAY=:1 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 GDK_BACKEND=x11 NO_AT_BRIDGE=1
Xvfb :1 -screen 0 1280x800x24 -nolisten tcp >/dev/null 2>&1 & sleep 2
openbox >/dev/null 2>&1 & sleep 1
python3 docker/native/arrow-window-spike.py target/debug/quiver-arrow-window docker/native/evidence/arrow-window-spike.png

#!/usr/bin/env bash
# Brings up the virtual X display, a desktop session, VNC, and noVNC's
# websocket bridge, then execs whatever the container was told to run (the
# scenario driver script, or an interactive shell for manual poking), so the
# real driving work happens as the container's own foreground process rather
# than racing it in the background. Adapted from Crowbar's docker/dev-linux
# start.sh.
set -euo pipefail

DISPLAY_NUM="${DISPLAY#:}"
XVFB_PIDFILE="/tmp/xvfb.pid"

cleanup() {
    kill "$(cat "$XVFB_PIDFILE" 2>/dev/null)" 2>/dev/null || true
}
trap cleanup EXIT

echo "[start.sh] launching Xvfb on ${DISPLAY} (${RESOLUTION})"
Xvfb "${DISPLAY}" -screen 0 "${RESOLUTION}" -nolisten tcp &
echo $! > "$XVFB_PIDFILE"

for _ in $(seq 1 50); do
    if xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done
if ! xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; then
    echo "[start.sh] Xvfb did not come up on ${DISPLAY}" >&2
    exit 1
fi

echo "[start.sh] starting xfce4-session"
dbus-launch --exit-with-session xfce4-session &

echo "[start.sh] starting x11vnc on port ${VNC_PORT}"
X11VNC_ARGS=(-display "${DISPLAY}" -forever -shared -rfbport "${VNC_PORT}" -nolookup -noxdamage)
if [ -n "${VNC_PASSWORD:-}" ]; then
    X11VNC_ARGS+=(-passwd "${VNC_PASSWORD}")
else
    echo "[start.sh] VNC_PASSWORD not set -- running with no VNC auth (dev-only; keep this port off any untrusted network)"
    X11VNC_ARGS+=(-nopw)
fi
x11vnc "${X11VNC_ARGS[@]}" &

echo "[start.sh] starting noVNC/websockify on port ${NOVNC_PORT} -> 127.0.0.1:${VNC_PORT}"
websockify --web /opt/novnc "${NOVNC_PORT}" "127.0.0.1:${VNC_PORT}" &

# Give the desktop session a moment to actually paint before anything tries
# to screenshot or drive it.
sleep 2

if [ "$#" -eq 0 ]; then
    echo "[start.sh] no command given, keeping the box up (VNC at :${NOVNC_PORT})"
    wait -n
else
    echo "[start.sh] running: $*"
    "$@"
fi

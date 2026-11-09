#!/usr/bin/env python3
"""Spike: run quiver-arrow-window under Xvfb, serve it a page through the host IPC, screenshot it."""
import base64, json, subprocess, sys, os, time

binary, out_png = sys.argv[1], sys.argv[2]
p = subprocess.Popen([binary, "arrow-app://spike.localhost/", "Spike arrow"], stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, text=True)
html = "<html><body style='font:32px sans-serif;background:#0b6;color:#fff'><h1>Served through the host IPC</h1></body></html>"
served = False
for line in p.stdout:
    msg = json.loads(line)
    print("window ->", msg["type"], msg.get("uri", ""), flush=True)
    if msg["type"] == "protocol_request":
        resp = {"type": "protocol_response", "id": msg["id"], "status": 200,
                "headers": [["content-type", "text/html"]],
                "body": base64.b64encode(html.encode()).decode()}
        p.stdin.write(json.dumps(resp) + "\n"); p.stdin.flush()
        served = True
        break
# Give WebKit one paint; the harness replaces this with the bridge's first-frame signal.
time.sleep(4)
os.system(f"import -window root {out_png}")
p.stdin.write(json.dumps({"type": "close"}) + "\n"); p.stdin.flush()
print("closed ->", p.wait(timeout=10), "served" if served else "NOT SERVED")

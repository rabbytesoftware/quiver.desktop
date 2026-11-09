#!/usr/bin/env python3
"""Spike: run quiver-arrow-window under Xvfb, serve it a page through the host IPC, screenshot it.

No timing guess for "the page has painted": the served page reports back through the same IPC. It requests
/painted from two nested requestAnimationFrame callbacks, i.e. after the browser has committed a frame with the
page's content, and the screenshot is taken on that request.
"""
import base64, json, subprocess, sys, os

binary, out_png = sys.argv[1], sys.argv[2]
p = subprocess.Popen([binary, "arrow-app://spike.localhost/", "Spike arrow"], stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, text=True)
html = ("<html><body style='font:32px sans-serif;background:#0b6;color:#fff'><h1>Served through the host IPC</h1>"
        "<script>requestAnimationFrame(()=>requestAnimationFrame(()=>fetch('/painted')))</script></body></html>")


def respond(msg, status, headers, body):
    resp = {"type": "protocol_response", "id": msg["id"], "status": status, "headers": headers,
            "body": base64.b64encode(body.encode()).decode()}
    p.stdin.write(json.dumps(resp) + "\n")
    p.stdin.flush()


served = painted = False
for line in p.stdout:
    msg = json.loads(line)
    print("window ->", msg["type"], msg.get("uri", ""), flush=True)
    if msg["type"] != "protocol_request":
        continue
    if msg["uri"].endswith("/painted"):
        respond(msg, 204, [], "")
        painted = True
        break
    respond(msg, 200, [["content-type", "text/html"]], html)
    served = True
if painted:
    os.system(f"import -window root {out_png}")
p.stdin.write(json.dumps({"type": "close"}) + "\n")
p.stdin.flush()
print("closed ->", p.wait(timeout=10), "served" if served else "NOT SERVED", "painted" if painted else "NOT PAINTED")
sys.exit(0 if painted else 1)

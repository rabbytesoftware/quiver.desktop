#!/usr/bin/env python3
"""The E2E Echo App's interface: server.py SOCKET [DELAY_SECONDS].

Sleeps DELAY_SECONDS, then serves HTTP on the unix socket SOCKET. GET
/headers answers the request line and headers it received as JSON; any other
path answers a small HTML page.
"""

import http.server
import json
import os
import signal
import socketserver
import sys
import time

SOCKET = sys.argv[1]
time.sleep(float(sys.argv[2]) if len(sys.argv) > 2 else 0)

PAGE = (
    b"<!doctype html><html><head><title>E2E Echo App</title></head>"
    b"<body><h1 id=app>E2E Echo App</h1></body></html>"
)


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):  # unix peers have no address to log
        pass

    def answer(self):
        if self.path.startswith("/headers"):
            seen = {"method": self.command, "path": self.path,
                    "headers": {k.lower(): v for k, v in self.headers.items()}}
            body, ctype = json.dumps(seen).encode(), "application/json"
        else:
            body, ctype = PAGE, "text/html; charset=utf-8"
        length = int(self.headers.get("Content-Length") or 0)
        if length:
            self.rfile.read(length)
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    do_GET = do_POST = do_HEAD = answer


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


def leave(*_):
    try:
        os.remove(SOCKET)
    finally:
        os._exit(0)


signal.signal(signal.SIGTERM, leave)
signal.signal(signal.SIGINT, leave)
if os.path.exists(SOCKET):
    os.remove(SOCKET)
server = Server(SOCKET, Handler)
os.chmod(SOCKET, 0o600)
server.serve_forever()

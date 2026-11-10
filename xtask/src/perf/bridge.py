"""Minimal client for the e2e bridge (newline-delimited JSON-RPC 2.0 over TCP, see docs/e2e.md)."""
import json
import socket


class Bridge:
    def __init__(self, port):
        self.sock = socket.create_connection(("127.0.0.1", port))
        self.file = self.sock.makefile("rwb")
        self.next_id = 0

    def call(self, method, params=None, timeout=120):
        self.next_id += 1
        self.sock.settimeout(timeout)
        self.file.write((json.dumps({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params or {}}) + "\n").encode())
        self.file.flush()
        while True:
            reply = json.loads(self.file.readline())
            if reply.get("id") == self.next_id:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply.get("result")

    def signal(self, name, timeout_ms=120000):
        return self.call("wait.signal", {"signal": name, "timeout_ms": timeout_ms}, timeout=timeout_ms / 1000 + 5)

    def node(self, id_, timeout_ms=120000):
        return self.call("wait.node", {"id": id_, "present": True, "timeout_ms": timeout_ms}, timeout=timeout_ms / 1000 + 5)

    def frame(self):
        return self.call("ui.tree")["frame"]

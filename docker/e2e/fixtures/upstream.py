#!/usr/bin/env python3
"""A local stand-in for the GitHub endpoints quiver.core actually talks to.

WHY THIS EXISTS, AND WHY IT IS NOT A MOCK OF QUIVER'S OWN CODE.

Every one of the three scenarios needs an "upstream" that can gain a new
release mid-test. The real github.com cannot: quiver.desktop has zero
published releases, and this branch's ARROW.md is not pushed anywhere. So the
container resolves the three GitHub hostnames quiver.core reaches for to
127.0.0.1 (see hosts-patch in scenarios/lib.sh) and this server answers them,
behind a CA the container trusts.

Nothing in quiver.core is stubbed, patched or rebuilt for this. The binary
under test is byte-identical to a production build: it still derives
`https://raw.githubusercontent.com/{user}/{repo}/{ref}/ARROW.md` from its own
embedded metadata.yaml, still follows `/releases/latest` for its redirect, and
still runs the real fetch + sha256 verification on whatever comes back. Only
the machine's name resolution and trust store differ, which is the same thing
a corporate TLS-inspecting proxy does to every Go program on earth.

Endpoints, matched on the Host header:

  raw.githubusercontent.com
    GET /{user}/{repo}/{ref}/{path...}
        The manifest fetch path (resolvers/http.go -> host.RawFileURL).
        Served from state/raw/{user}/{repo}/{ref}/{path}.

  github.com
    GET /{user}/{repo}/releases/latest
        302 -> /{user}/{repo}/releases/tag/{latest}. This is the ENTIRE
        drift check for a tag-tracked arrow: providers/host.go's
        LatestRelease reads the Location header and nothing else, looking for
        the literal marker "/releases/tag/". Repointing the LATEST file is
        how a scenario publishes a new version.
    GET /{user}/{repo}/releases/download/{tag}/{asset}
    GET /{user}/{repo}/releases/latest/download/{asset}
        Release assets, and GitHub's own "always the newest release" alias
        for one. Served from state/releases/{user}/{repo}/{tag}/{asset}.
    GET  /{user}/{repo}/info/refs?service=git-upload-pack
    POST /{user}/{repo}/git-upload-pack
        Real git smart-HTTP, delegated to git's own http-backend CGI over
        state/git/{user}/{repo}.git. Needed because a globbed dependency
        constraint (quiver.desktop's `tools:` edge on
        quiver.core@stable-26.5*) is resolved by go-git's ls-remote against
        the clone URL, with no host-specific shortcut.

  api.github.com
    GET /repos/{user}/{repo}/releases/latest
    GET /repos/{user}/{repo}/releases/tags/{tag}
        The releases API shape install.sh selects assets from, and that the
        app's own resolver (src-tauri/src/release/mod.rs) reads when the user
        clicks Update on Quiver's own tile. Pretty-printed, with a nested
        uploader object and a real per-asset "digest", because that is what
        api.github.com sends -- and because the digest is the only checksum a
        quiver.desktop release publishes, so the update verifies against it.

Every request is logged as one line of JSON to the results directory, so a
scenario can prove after the fact that a download really was served from here
rather than skipped, cached or short-circuited.
"""

import hashlib
import http.server
import json
import os
import ssl
import subprocess
import sys
import threading
import time
import urllib.parse
from pathlib import Path

STATE = Path(os.environ.get("UPSTREAM_STATE", "/workspace/run/upstream"))
LOG_PATH = Path(os.environ.get("UPSTREAM_LOG", "/workspace/results/upstream.log"))
CERT = os.environ.get("UPSTREAM_CERT", "/workspace/run/tls/server.pem")
KEY = os.environ.get("UPSTREAM_KEY", "/workspace/run/tls/server.key")
PORT = int(os.environ.get("UPSTREAM_PORT", "443"))

_log_lock = threading.Lock()


def log_event(**fields):
    fields["ts"] = time.time()
    line = json.dumps(fields, sort_keys=True)
    with _log_lock:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        with LOG_PATH.open("a") as handle:
            handle.write(line + "\n")


def latest_tag(user, repo):
    marker = STATE / "releases" / user / repo / "LATEST"
    if not marker.is_file():
        return None
    return marker.read_text().strip() or None


def asset_path(user, repo, tag, asset):
    return STATE / "releases" / user / repo / tag / asset


def sha256_of(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class Handler(http.server.BaseHTTPRequestHandler):
    # HTTP/1.1 so git's smart protocol and Go's client both get keep-alive and
    # a Content-Length they can trust.
    protocol_version = "HTTP/1.1"
    server_version = "quiver-e2e-upstream/1.0"

    # -- plumbing ---------------------------------------------------------

    def log_message(self, fmt, *args):  # noqa: A003 - stdlib hook name
        # Silenced: every request is already recorded structurally by
        # log_event, and stderr noise drowns the scenario output.
        pass

    def vhost(self):
        return (self.headers.get("Host") or "").split(":")[0].lower()

    def send_bytes(self, status, body, content_type="application/octet-stream", extra=None):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        for key, value in (extra or {}).items():
            self.send_header(key, value)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def send_redirect(self, location):
        self.send_response(302)
        self.send_header("Location", location)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def send_text(self, status, text):
        self.send_bytes(status, text.encode(), "text/plain; charset=utf-8")

    def send_file(self, path, content_type="application/octet-stream"):
        if not path.is_file():
            self.send_text(404, f"not found: {path}")
            return False
        self.send_bytes(200, path.read_bytes(), content_type)
        return True

    # -- routing ----------------------------------------------------------

    def do_GET(self):  # noqa: N802 - stdlib hook name
        self.route("GET")

    def do_HEAD(self):  # noqa: N802
        self.route("GET")

    def do_POST(self):  # noqa: N802
        self.route("POST")

    def route(self, method):
        parsed = urllib.parse.urlparse(self.path)
        host = self.vhost()
        log_event(event="request", method=method, host=host, path=self.path)
        try:
            if host == "raw.githubusercontent.com":
                self.route_raw(parsed)
            elif host == "api.github.com":
                self.route_api(parsed)
            elif host == "github.com":
                self.route_github(method, parsed)
            else:
                self.send_text(404, f"no vhost for {host}")
        except BrokenPipeError:
            pass
        except (ssl.SSLEOFError, ssl.SSLZeroReturnError, ConnectionResetError) as exc:
            # The client hung up on us, which for a large asset is the NORMAL
            # end of a successful transfer: Go's http client reads the body it
            # was promised and closes, and this side then finds the TLS
            # connection gone while finishing up. Logged as a disconnect, not
            # an error, so a green run's log does not read like four failures.
            # Whether the bytes actually arrived is settled by the checksum the
            # scenario asserts on the downloaded file, not by this.
            log_event(event="client.disconnected", host=host, path=self.path, detail=repr(exc))
        except Exception as exc:  # noqa: BLE001 - a fixture must not die on one bad request
            log_event(event="error", host=host, path=self.path, error=repr(exc))
            try:
                self.send_text(500, f"upstream fixture error: {exc}")
            except Exception:  # noqa: BLE001
                pass

    def route_raw(self, parsed):
        parts = [p for p in parsed.path.split("/") if p]
        if len(parts) < 4:
            self.send_text(404, "raw path needs /user/repo/ref/file")
            return
        user, repo, ref, rest = parts[0], parts[1], parts[2], parts[3:]
        target = STATE.joinpath("raw", user, repo, ref, *rest)
        if self.send_file(target, "text/plain; charset=utf-8"):
            log_event(event="raw.served", user=user, repo=repo, ref=ref, file="/".join(rest))

    def route_api(self, parsed):
        parts = [p for p in parsed.path.split("/") if p]
        # /repos/{user}/{repo}/releases/{latest|tags/{tag}}
        if len(parts) < 5 or parts[0] != "repos" or parts[3] != "releases":
            self.send_text(404, "unsupported api path")
            return
        user, repo = parts[1], parts[2]
        if parts[4] == "latest":
            tag = latest_tag(user, repo)
        elif parts[4] == "tags" and len(parts) >= 6:
            tag = parts[5]
        else:
            tag = None
        if tag is None:
            self.send_bytes(404, b'{"message":"Not Found"}', "application/json")
            return
        directory = STATE / "releases" / user / repo / tag
        assets = []
        if directory.is_dir():
            for item in sorted(directory.iterdir()):
                if not item.is_file():
                    continue
                assets.append({
                    "name": item.name,
                    # Nested, exactly as the real document nests it. Not
                    # decoration: install.sh parses this without jq, and a
                    # nested object is what breaks a naive split on "{".
                    "uploader": {"login": "github-actions[bot]", "type": "Bot"},
                    "size": item.stat().st_size,
                    # GitHub's own record of what it stored, reported per asset
                    # since 2025 and populated for everything uploaded since.
                    # This is what quiver.desktop's Update button verifies
                    # against: this repository's release workflow publishes no
                    # checksum manifest, so without it there is nothing to
                    # check and the app refuses the update rather than
                    # installing something it cannot verify.
                    "digest": "sha256:" + sha256_of(item),
                    "browser_download_url":
                        f"https://github.com/{user}/{repo}/releases/download/{tag}/{item.name}",
                })
        # Pretty-printed, because api.github.com pretty-prints. A fixture that
        # answered on one line would hide every line-oriented parsing bug the
        # real document can provoke.
        body = json.dumps(
            {"tag_name": tag, "name": tag, "assets": assets}, indent=2
        ).encode()
        self.send_bytes(200, body, "application/json")

    def route_github(self, method, parsed):
        parts = [p for p in parsed.path.split("/") if p]
        if len(parts) < 3:
            self.send_text(404, "unsupported github path")
            return
        user, repo, tail = parts[0], parts[1], parts[2:]

        if tail[0] == "releases":
            self.route_releases(user, repo, tail[1:])
            return
        if tail == ["info", "refs"] or tail == ["git-upload-pack"]:
            self.route_git(method, parsed, user, repo, tail)
            return
        self.send_text(404, "unsupported github path")

    def route_releases(self, user, repo, tail):
        # /releases/latest                      -> the drift-check redirect
        if tail == ["latest"]:
            tag = latest_tag(user, repo)
            if tag is None:
                self.send_text(404, "no releases")
                return
            location = f"https://github.com/{user}/{repo}/releases/tag/{tag}"
            log_event(event="releases.latest", user=user, repo=repo, tag=tag)
            self.send_redirect(location)
            return

        # /releases/tag/{tag}                   -> the page the redirect lands on
        if len(tail) == 2 and tail[0] == "tag":
            self.send_text(200, f"release {tail[1]}")
            return

        # /releases/latest/download/{asset}     -> GitHub's newest-release alias
        if len(tail) == 3 and tail[0] == "latest" and tail[1] == "download":
            tag = latest_tag(user, repo)
            if tag is None:
                self.send_text(404, "no releases")
                return
            self.send_redirect(
                f"https://github.com/{user}/{repo}/releases/download/{tag}/{tail[2]}"
            )
            return

        # /releases/download/{tag}/{asset}
        if len(tail) == 3 and tail[0] == "download":
            tag, asset = tail[1], tail[2]
            path = asset_path(user, repo, tag, asset)
            if self.send_file(path):
                log_event(
                    event="asset.served", user=user, repo=repo, tag=tag,
                    asset=asset, bytes=path.stat().st_size,
                )
            return

        self.send_text(404, "unsupported releases path")

    # -- git smart HTTP ---------------------------------------------------

    def route_git(self, method, parsed, user, repo, tail):
        """Delegate to git's own http-backend CGI.

        Reimplementing the pkt-line advertisement by hand would be a second
        implementation of git's wire protocol to get wrong; http-backend is
        the same program every git hosting box runs behind a web server.
        """
        repo_dir = STATE / "git" / user / f"{repo}.git"
        if not repo_dir.is_dir():
            self.send_text(404, f"no such repository: {user}/{repo}")
            return

        env = dict(os.environ)
        env.update({
            "GIT_PROJECT_ROOT": str(repo_dir.parent),
            "GIT_HTTP_EXPORT_ALL": "1",
            "PATH_INFO": "/" + f"{repo}.git/" + "/".join(tail),
            "QUERY_STRING": parsed.query,
            "REQUEST_METHOD": method,
            "REMOTE_ADDR": self.client_address[0],
            "CONTENT_TYPE": self.headers.get("Content-Type", ""),
            "HTTP_CONTENT_ENCODING": self.headers.get("Content-Encoding", ""),
            "SERVER_PROTOCOL": "HTTP/1.1",
            "GIT_HTTP_MAX_REQUEST_BUFFER": "100M",
        })
        length = self.headers.get("Content-Length")
        body = b""
        if length:
            env["CONTENT_LENGTH"] = length
            body = self.rfile.read(int(length))

        proc = subprocess.run(
            ["git", "http-backend"],
            input=body,
            capture_output=True,
            env=env,
            check=False,
        )
        if proc.returncode != 0:
            log_event(event="git.error", user=user, repo=repo,
                      stderr=proc.stderr.decode(errors="replace")[:2000])
            self.send_text(500, "git http-backend failed")
            return

        head, _, payload = proc.stdout.partition(b"\r\n\r\n")
        status = 200
        headers = {}
        for raw in head.split(b"\r\n"):
            if not raw:
                continue
            key, _, value = raw.decode("latin-1").partition(":")
            value = value.strip()
            if key.lower() == "status":
                status = int(value.split()[0])
            else:
                headers[key] = value
        content_type = headers.pop("Content-Type", "application/octet-stream")
        log_event(event="git.served", user=user, repo=repo, path="/".join(tail), status=status)
        self.send_bytes(status, payload, content_type, headers)


class ThreadingHTTPSServer(http.server.ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def main():
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(certfile=CERT, keyfile=KEY)

    server = ThreadingHTTPSServer(("0.0.0.0", PORT), Handler)
    server.socket = context.wrap_socket(server.socket, server_side=True)
    log_event(event="started", port=PORT, state=str(STATE))
    print(f"[upstream] listening on :{PORT} (state={STATE})", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())

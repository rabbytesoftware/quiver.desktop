# Arrow apps

## What an arrow app is

An arrow can ship a web interface. The daemon serves it under `/v0/ui/<namespace>/`. Quiver Desktop shows that interface inside the app, in an iframe, so you can use an arrow (a chat, a dashboard) without leaving the shell. The arrow's page is ordinary web content: HTML, scripts, styles, `fetch` calls to its own origin and, through a shim, WebSockets to its own server.

## Request path

The iframe loads `arrow-app://<host>/`, where the host is `<hex>.localhost`. On Windows, wry rewrites that origin to `http://arrow-app.<hex>.localhost/`, and the app accepts both forms. The `.localhost` suffix is what keeps the Windows origin a secure context: a plain `http://arrow-app.<hex>` origin is not one, and pages there lose `crypto.randomUUID`, `crypto.subtle` and the clipboard API.

1. The shell asks Rust to register the arrow's namespace (`arrow_app_host`). The host is the first 32 lowercase hex characters of the SHA-256 of the namespace followed by `.localhost`, so it is DNS-safe and stable between launches. The bare hex label on its own does not resolve.
2. The page requests `arrow-app://<host>/some/path?q=1`. The `arrow-app` URI scheme handler looks the host up. Hosts nobody registered answer 404.
3. The handler rewrites the request to `/v0/ui/<percent-encoded namespace>/some/path?q=1` and sends it through the active connection (local, SSH or whatever transport is selected). The namespace is encoded as a single path segment.
4. The response comes back through the same transport and is hardened before it reaches the page.

No port is opened on this machine, and the bearer token is added on the Rust side, so it never reaches the page. Requests with `..` or encoded dot or backslash segments in the path are refused with 400. The headers `Origin`, `Referer`, `Cookie`, `Authorization`, `Host` and `Accept-Encoding` are dropped on the way to the daemon, and the encoding is forced to `identity`.

Successful HTML responses get `<script src="/__arrow/shim.js"></script>` inserted right after `<head>` (else after `<html>`, else after the doctype, else at the start). The shim itself is served by the handler from a copy built into the binary. Any other path under `/__arrow/` is a 404.

## Security model

An arrow's page is untrusted. It shares a webview process with the app's own pages, so the boundaries are explicit.

- **No Tauri IPC in the page.** The arrow page can not call Tauri commands. Everything it needs goes through the shell, which relays for it. There is no `window.arrow` object either.
- **Handler-set CSP and header hardening.** Every `arrow-app` response, including errors, carries the handler's Content-Security-Policy: `default-src 'self'`, inline scripts and styles allowed (Next static exports need them), `connect-src 'self'`, `frame-src 'none'`, `form-action 'self'` and `base-uri 'self'`. The arrow's own CSP, `Content-Security-Policy-Report-Only`, `X-Frame-Options`, CORS headers and `Set-Cookie` are removed. `X-Content-Type-Options: nosniff` and `Referrer-Policy: no-referrer` are added. `connect-src 'self'` is what stops the page from calling `quiver://localhost`.
- **`quiver://` origin allowlist.** The `quiver://` proxy answers CORS for the app's own pages and adds the bearer token. It now refuses any request whose `Origin` is not one of the app's own (`tauri://localhost`, `http://tauri.localhost`, `https://tauri.localhost`, `http://localhost:1420`), with a 403. Arrow origins are not on the list. Requests with no `Origin` are allowed. The arrow CSP stops the page from fetching `quiver://`, but the CSP does not cover a frame navigating itself, so two more rules close that path: every `quiver://` response (successes, errors, the preflight and the 403 refusals) carries `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY`, so no `quiver://` document can render inside an iframe, and `quiver://` refuses any path under `/v0/ui/` with a 403 before a connection is resolved (the shell never loads arrow interfaces through it).
- **Scoped WebSocket commands.** Arrow sockets use `arrow_ws_open`, `arrow_ws_send` and `arrow_ws_close`, not the generic `ws_open`. The target path is built in Rust as `/v0/ui/<that arrow's namespace>/...` from the registered host, never from the page. Paths that do not start with a single `/`, or contain `..`, backslashes, encoded dots or backslashes, control characters, spaces or `#`, are rejected. Connection keys are prefixed with the host, so two arrows using the same id never collide.
- **Per-arrow origins.** Each arrow has its own host, so each has its own origin and its own storage (`localStorage`, IndexedDB). One arrow can not read another's data.
- **Shim hello and per-document ids.** The shim sends a `hello` when a document loads, which makes the shell close any sockets the previous document in that frame left behind. Socket ids carry a random per-document nonce so a new document's ids can not collide with a stale one.
- **Message authority.** The shell trusts a message only if `event.source` is the iframe registered for a host and `event.origin` is exactly that host's own origin (`arrow-app://<host>`, or `http://arrow-app.<host>` on Windows), and the message only touches that host's sockets. Ids are length limited, WebSocket paths longer than 2048 characters are refused with a close 1006, and each arrow may hold at most 8 sockets.
- **Outbound trust.** A frame can navigate itself to a foreign document while keeping the same window, so the shell sends frames to a host only while it trusts that host. An origin-checked `hello` grants trust. When the iframe fires `load` and no origin-checked `hello` arrived for that document within a short grace period (200 ms), the shell closes the host's sockets and stops sending to it until the next `hello`. Replies still use `'*'` as the target origin, because custom-scheme origins are not verified as `postMessage` targets.
- **Only WebSocket text frames cross the bridge.** The shell understands three message types from a page (`ws-open`, `ws-send`, `ws-close`) and nothing else. Every message must carry the shim tag and pass the source and origin checks above; anything else is dropped.

## Known limits

- **No cookies.** Cookies do not work on custom schemes, and `Set-Cookie` is stripped anyway. Arrows must not rely on them.
- **Text WebSocket frames only.** Binary frames are not supported. The shim logs an error and drops a non-string `send`.
- **No response streaming.** A response is buffered whole before the page sees it, so server-sent events and long streaming bodies do not work. A request that takes more than 300 seconds ends in a 504.
- **Windows is untested at runtime.** The `http://arrow-app.<hex>.localhost` form is handled and unit tested, but it has not been run on a Windows machine.
- **No `blob:` media and no WebAssembly.** The handler CSP has no `media-src`, so audio and video fall back to `default-src 'self'` and `blob:` media URLs are blocked. It has no `'wasm-unsafe-eval'`, so WebAssembly can not be compiled.
- **Child frame behaviour is only verified on macOS.** That iframes get no Tauri IPC or init scripts, and that each custom-scheme origin has its own `localStorage`, was checked on WKWebView. WebKitGTK (Linux) and WebView2 (Windows) have not been checked.
- **At most 4 apps are kept alive.** The shell keeps recently used arrow apps mounted and hidden so state survives switching. Opening a fifth drops the least recently used one that is not visible.
- **At most 8 sockets per arrow.** Further `new WebSocket` calls close at once with code 1006.
- **Arrow pages can navigate their own frame away.** The sandbox blocks top navigation and popups, but nothing stops a page from navigating its own iframe to an external site, and links are not handed to the system browser. A document that is not the arrow's gets no Tauri IPC, its messages fail the origin check, and the shell stops relaying to it (see Outbound trust). It can not render `quiver://` content, which forbids framing. For up to the 200 ms grace period after such a load, frames already in flight may still be posted into the frame.

## Trying one locally

The end to end walkthrough, with the chat arrow served by a local daemon and opened in the app, lives in the quiver.chat plan, in its integration task: `docs/superpowers/plans/2026-10-04-arrow-apps-chat.md`. That path is only a pointer, and it is not part of this repository's tracked docs.

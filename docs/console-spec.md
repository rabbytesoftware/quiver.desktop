# Console and build indicator

Design: https://claude.ai/artifact/YSPPYCZcCTa89Ai5h8859L (the approved canvas; the running app is the refined version of it).

The rail's top row shows which builds are running, `core` over `app`. Clicking it drops a quake-style console from the top of the content: the connected daemon's log, and a prompt that runs the daemon's own CLI commands. This file is desktop's half of the contract; quiver.core's `docs/spec/console.md` describes the same wire format.

## 1. What desktop does and does not own

| Daemon (quiver.core) | Desktop (here) |
|---|---|
| The command grammar and which commands are reachable | Nothing. There is **no command list in this repo**: help and Tab completion read `GET /v0/console/commands`. |
| Redacting secrets from log records | Never logging frame contents to its own log (`console_exec` and the bridge log no payloads). |
| Limits on a command line, its output, its duration and its concurrency | Moving bytes, bounding its own memory, cancelling by closing the connection. |
| Auth (bearer token over TCP, trusted socket locally) | Using the active connection's transport, which already carries it. |

Default-deny is the daemon's. Desktop's only policy is to send the line exactly as typed and to show what comes back.

## 2. Capability

`GET /versions` (not under `/v0`) is read when a connection becomes ready. The console is offered only when `features` contains `console.v1`:

- `supported`: the stream opens while the console is open; the prompt works.
- `unsupported` (a reply without the flag): the panel says so in one line and offers no controls. Running a command is refused client-side with the same message.
- `unknown` (no answer yet, or the daemon is unreachable): the prompt is offered and a command is tried; the daemon or the transport answers. Never shown as "unsupported" about a daemon that has not been heard from.

The indicator shows build information in every case.

## 3. The build indicator

Two lines in the rail's top row, right of the traffic-light gutter (and of the connection switcher, left of back/forward):

```
core  nightly 10-04 13:47
app   nightly 10-04 14:02
```

| Build | At rest | On hover or focus |
|---|---|---|
| rolling (nightly) | `nightly MM-DD HH:mm`, the build time in the viewer's zone | `nightly <7-char commit>` |
| stable | `stable 26.5.1` | `stable <commit>` |
| beta, hotfix | `beta 26.5 #4`: series and the tag's trailing build count | `beta <commit>` |
| unstamped | `dev` | `dev <commit>` when known |
| not yet known | `—` | |

Narrow rail: below 160px the time of day is dropped (`nightly 10-04`). Releases are already short. The decision on rail width is in the PR description: the default stays 246px and the indicator degrades, rather than every user's rail growing.

Where the values come from:

- **core**: `/versions` `version`, `commit`, `built_at`, `channel`. `channel` is the pipeline's raw value (`stable`, `beta`, `hotfix`, `nightly-latest`); any `nightly*` is the rolling channel. A release's version arrives as its tag (`beta-26.5-4`) or bare (`26.5-4`).
- **app**: `get_build_stamp` (compile time, `src-tauri/build.rs`) and `VITE_QUIVER_BUILD_CHANNEL`:
  - `commit`: `QUIVER_DESKTOP_STAMP_COMMIT`, else `GITHUB_SHA`, else `git rev-parse HEAD`.
  - `built_at`: `SOURCE_DATE_EPOCH`, else the time the build script ran.
  - `label`: the release tag in any channel, from the `build-label` input of `.github/actions/build-tauri` (stable falls back to `release-tag`). Display only; it is never announced to quiver.core as a ref. Nightly passes none and is identified by commit and time.

The traffic-light gutter exists only on macOS with the rail on the left (`railOwnsControls`). Back/forward and the connection switcher stay in the same row.

## 4. Wire contract

### 4.1 `GET /v0/console/logs` (WebSocket, through the generic bridge)

Opened with `level=debug&replay=500` and, on reconnect, `since=<highest seq shown>`. Frames, one JSON object each:

```json
{"type":"log","seq":412,"time":"2026-10-04T14:02:14.390123Z","level":"warn","component":"release","msg":"channel lookup slow","fields":{"ns":"github.com/char2cs/crowbar","took":"1.8s","retry":1},"fields_truncated":false}
{"type":"ready","seq":412}
{"type":"ready","seq":3,"reset":true}
{"type":"gap","dropped":37}
```

- Order: replay frames, **one** `ready`, then live frames.
- **The replay is held until its `ready`**, then folded in together. A `ready` with `reset: true` means the daemon restarted (or `since` was newer than its newest record): the daemon lines already shown belong to another process and are discarded before the replay is shown, with a "Daemon restarted" note. The user's own commands and their output stay.
- Frames can arrive before `onopen` (the bridge forwards as soon as the socket is up), so a replay begins when an attempt begins, not when it opens.
- A record at or below the cursor is dropped, so a replay racing the live feed shows each line once.
- A dropped connection reconnects with backoff (1s doubling to 30s) from the cursor; a reconnect shows "Reconnecting…" in the prompt row.
- Switching connection stops the stream, discards the buffer and re-asks the new daemon.
- A frame that does not decode is shown as raw text; nothing throws.

### 4.2 `GET /v0/console/commands`

`data.commands[]`: `path`, `short`, `usage`, `aliases`, and `flags[{name, shorthand, usage, takes_value}]`. Used for `help` and Tab completion: command words, and the flags of the command a line is addressed to (`--ye` → `--yes `; a flag that takes a value completes to `--output=`).

### 4.3 `POST /v0/console/exec`

Request `{"line":"install github.com/char2cs/crowbar"}`, no leading `quiver`. The reply is NDJSON that the `quiver://` proxy cannot carry (it returns a response whole), so Rust is the client: `console_exec` posts through `Transport::request_stream` and pushes each line, verbatim, down a Tauri channel.

```json
{"type":"out","stream":"stdout","data":"resolving …\n"}
{"type":"exit","code":0,"error":""}
```

- Every accepted run ends in one `exit` frame. Codes are the CLI's: 0, 1, 2 (usage), 3 (daemon unreachable), 130 (interrupted). A non-zero code is shown as an error-styled final line with the code and the daemon's message.
- A refusal before execution is a plain error envelope; Rust turns it into one `{"type":"error","status":403,"message":"…"}` frame, shown as "Refused (403): …" in the daemon's own words. A failure to reach the daemon is `status: 0`.
- A connection that ends before an `exit` frame becomes one `error` frame, so the console never waits on a command that is gone.
- **Confirmations never prompt.** A destructive command without `--yes`/`-y` refuses at once ("requires --yes/-y when not running interactively") on stderr. The UI just shows that output; it never waits on a prompt.
- Output is shown a line at a time; a line split across frames is joined; ANSI escapes are stripped; a line over 1 MiB is replaced by a marker.
- Cancelling (or a page reload) closes the connection, even while the command is silent, which cancels it on the daemon.
- Only a bare `help` and `clear` are handled in the client. `help <command>` goes to the daemon.

## 5. The console

- Layout: log and prompt, nothing else. No title bar, filters, chips or shortcut hints. Escape closes; so does the indicator.
- A log line: `time  LEVEL  message key=value …` (no column for the component: most lines have none and an empty column left a gap after the level; it is in the raw JSON one click away). Only warn and error colour their level and message. Values: numbers and durations blue, booleans green, paths underlined, `err`/`error` red. Clicking a line shows the record as JSON.
- The buffer holds 5000 entries, oldest dropped first, in a virtualized list that follows the newest line until the user scrolls up.
- History: Up/Down, 200 entries, kept across connection switches.
- Follows the app theme (light and dark).

## 6. Tests

- Pure and fully tested: indicator text for every channel, hover and narrow rail (`lib/build-info.test.ts`); frame decoding (`frames.test.ts`); capped buffer; history; command parsing and completion (`commands.test.ts`); `/versions` (`versions.test.ts`).
- Behaviour: the stream's reconnect and backoff, the store (cursor, reset, cap), the controller (capability, replay-until-ready, reset, switching connection, running a command), the components, and the rail row.
- Rust: build stamp validation, `request_stream` over unix and TCP (including cancel while the daemon is silent), the NDJSON splitter and `run_exec`.
- The mock backend speaks the daemon's real surface (the same grammar, refusals and `--yes` behaviour), so `make dev-mock` exercises the whole feature.
- `e2e/scenarios/console.spec.ts` drives the built app against a real daemon.

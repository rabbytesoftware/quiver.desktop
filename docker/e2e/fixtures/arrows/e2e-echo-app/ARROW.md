# E2E Echo App

A `listen` interface that waits a few seconds before it binds, so the spec can
see an arrow app that is open but not ready yet, and that answers
`/headers` with the request headers it received, so the spec can prove what
the daemon and the desktop strip on the way.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Echo App"
  description: "Binds its interface socket late and echoes request headers."
  version: "1.0"

targets:
  "*":
    lifecycle:
      install:
        - type: fetch
          url: https://github.com/rabbytesoftware/e2e-echo-app/releases/download/v1/server.py
          to: ${INSTALL_PATH}/server.py
          title: "Download the server"
          timeout: "1m"
      execute:
        - type: ui
          listen: [unix]
          path: /
          title: "E2E Echo App"
        - type: run
          command: 'python3 ./server.py "${ARROW_UI_LISTEN}" 5'
          title: "Serve"
          timeout: "3100s"
      stop:
        - type: signal
          signal: graceful
          title: "Stop"
          timeout: "5s"
          exit_on_failure: false
```

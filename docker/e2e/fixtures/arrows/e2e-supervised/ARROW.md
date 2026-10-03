# E2E Supervised

A long-running process for the E2E box to watch while quiver.core replaces itself.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Supervised"
  description: "Sleeps for a long time so a scenario can check its process survives a quiver.core restart."
  version: "1.0"

targets:
  "*":
    lifecycle:
      execute:
        - type: run
          command: "sleep 3000"
          title: "Execute"
          timeout: "3100s"
          exit_on_failure: true
      stop:
        - type: signal
          signal: graceful
          title: "Stop"
          timeout: "5s"
          exit_on_failure: false
```

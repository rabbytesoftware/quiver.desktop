# E2E Slow

A three-step install that waits in its second step, for the E2E box to watch while it runs.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Slow"
  description: "A three-step install that waits in the second step."
  version: "1.0"

targets:
  "*":
    lifecycle:
      install:
        - type: run
          command: "echo prepared"
          title: "Prepare"
          timeout: "30s"
        - type: run
          command: "sleep 20"
          title: "Settle"
          timeout: "60s"
        - type: run
          command: "echo finished"
          title: "Finish"
          timeout: "30s"
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
      uninstall:
        - type: run
          command: "echo removed"
          title: "Uninstall"
          timeout: "30s"
```

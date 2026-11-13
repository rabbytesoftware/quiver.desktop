# E2E Form

An arrow with a bounded number and a select, so the Settings modal has something to validate.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Form"
  description: "Declares a bounded number and a select variable."
  version: "1.0"

variables:
  - name: "E2E_PORT"
    description: "A number between 1 and 10."
    type: "number"
    default: "5"
    min: 1
    max: 10
  - name: "E2E_MODE"
    description: "One of the listed modes."
    type: "select"
    default: "fast"
    values: ["fast", "slow"]

targets:
  "*":
    lifecycle:
      install:
        - type: run
          command: "echo ${E2E_PORT} ${E2E_MODE}"
          title: "Install"
          timeout: "30s"
      uninstall:
        - type: run
          command: "echo removed"
          title: "Uninstall"
          timeout: "30s"
```

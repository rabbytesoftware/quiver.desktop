# E2E Required

An arrow whose update needs a value the caller must supply.

```arrow
schema: "arrow@v0"

metadata:
  name: "E2E Required"
  description: "Declares a required variable its update step expands, for the E2E box to leave empty."
  version: "1.0"

variables:
  - name: "E2E_TOKEN"
    description: "A value the update step needs. It has no default."

targets:
  "*":
    lifecycle:
      update:
        - type: run
          command: "echo updating with ${E2E_TOKEN}"
          title: "Update"
          timeout: "30s"
          exit_on_failure: true
```

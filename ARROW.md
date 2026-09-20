# Quiver

The Quiver desktop application — this file is quiver.desktop's own arrow
manifest, letting it appear in the catalog like any other arrow.

```arrow
schema: "arrow@v0"

metadata:
  name: "Quiver"
  description: "The Quiver desktop application."
  version: "0.1"
  license: "GPL-3.0"
  url: "https://github.com/rabbytesoftware/quiver.desktop"
  maintainers:
    - name: "Rabbyte Software"
      url: "https://char2cs.net"
  credits:
    - name: "Rabbyte Software"

variables:
  - name: "QUIVER_DESKTOP_DETECT_COMMAND"
    description: "Shell command the preinstalled check runs to detect an existing Quiver Desktop install on this platform."
    default: "command -v quiver-desktop"

targets:
  "*":
    requirements:
      cpu_cores: 1
      ram_gb: 1
      disk_gb: 1
    tools:
      - "github.com/rabbytesoftware/quiver.core@v26.*"
    lifecycle:
      preinstalled:
        - type: run
          title: "Detect an existing Quiver Desktop install"
          command: "${QUIVER_DESKTOP_DETECT_COMMAND}"
          timeout: "10s"
          exit_on_failure: false
```

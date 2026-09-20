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
    description: "Shell command the preinstalled check runs on Unix (sh -c) to detect an existing Quiver Desktop install."
    default: "command -v quiver-desktop"
  - name: "QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS"
    description: "Command the preinstalled check runs on Windows (cmd.exe /C) to detect an existing Quiver Desktop install."
    default: "where quiver-desktop"

targets:
  "*":
    requirements:
      cpu_cores: 1
      ram_gb: 1
      disk_gb: 1
    tools:
      - "github.com/rabbytesoftware/quiver.core@stable-26.5*"
    lifecycle:
      preinstalled:
        - type: run
          title: "Detect an existing Quiver Desktop install"
          command:
            default: "${QUIVER_DESKTOP_DETECT_COMMAND}"
            "windows/amd64": "${QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS}"
            "windows/arm64": "${QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS}"
          timeout: "10s"
          exit_on_failure: false
```

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
  # These look for what the build actually ships, which is two different names.
  # tauri.conf.json sets no `mainBinaryName`, so Tauri keeps cargo's output
  # name for the executable -- `quiverdesktop`, from src-tauri/Cargo.toml's
  # [package] name. `productName` ("Quiver") names the BUNDLE around it: the
  # macOS .app, and the Windows install directory. There is no `quiver-desktop`
  # anywhere; a check for one could never pass on any platform.
  - name: "QUIVER_DESKTOP_DETECT_COMMAND"
    description: >-
      Command the preinstalled check runs on Linux (sh -c) to detect an existing
      Quiver Desktop install. The .deb puts the main binary on PATH as
      /usr/bin/quiverdesktop. The AppImage is a portable single file the user
      keeps wherever they downloaded it, so it is deliberately not detected
      here: there is no convention to check, and guessing one would report
      installs that are not there.
    default: "command -v quiverdesktop"
  - name: "QUIVER_DESKTOP_DETECT_COMMAND_DARWIN"
    description: >-
      Command the preinstalled check runs on macOS (sh -c). A .app bundle is
      never on PATH, so this tests for the bundle directory itself under both
      places a user installs one. $HOME is the shell's, not a Quiver variable --
      only ${...} is expanded before the command reaches sh.
    default: 'test -d /Applications/Quiver.app || test -d "$HOME/Applications/Quiver.app"'
  - name: "QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS"
    description: >-
      Command the preinstalled check runs on Windows (cmd.exe /C). The app is
      on no PATH, so this tests for the executable where the published NSIS
      setup.exe puts it: %LOCALAPPDATA%\Quiver, Tauri's default because its
      nsis installMode defaults to currentUser.

      Deliberately contains NO double quotes, which is a hard constraint and
      not a style choice. quiver.core spawns this as exec.Command("cmd.exe",
      "/C", joined) with no SysProcAttr.CmdLine, so Go escapes the string with
      syscall.EscapeArg -- every " becomes \" -- and cmd.exe's /C rule then
      strips the outer quotes and reads \" as a literal backslash plus a quote
      toggle, mangling the path. A quote-free string round-trips through both
      untouched. The cost is that a path containing a space cannot be
      expressed, which is why the per-machine MSI root (%ProgramFiles% is
      always "C:\Program Files") is NOT checked here; override this variable
      to detect a per-machine or relocated install.
    default: 'if exist %LOCALAPPDATA%\Quiver\quiverdesktop.exe (exit 0) else (exit 1)'

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
          # Exact os/arch keys, never a "darwin/*" or "windows/*" glob.
          # quiver.core resolves a step's Overrideable fields with a plain
          # exact map lookup (step.Overrideable.Resolve), not the glob-aware
          # path.Match resolution the spec documents and Exports actually gets
          # -- so a glob key silently falls through to `default` on every
          # target. Established by Task 3.1 for the Windows keys and verified
          # again here; the `default` is therefore Linux's, not a shared Unix
          # one.
          command:
            default: "${QUIVER_DESKTOP_DETECT_COMMAND}"
            "darwin/amd64": "${QUIVER_DESKTOP_DETECT_COMMAND_DARWIN}"
            "darwin/arm64": "${QUIVER_DESKTOP_DETECT_COMMAND_DARWIN}"
            "windows/amd64": "${QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS}"
            "windows/arm64": "${QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS}"
          timeout: "10s"
          exit_on_failure: false
```

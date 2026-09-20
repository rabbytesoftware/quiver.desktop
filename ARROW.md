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
      on no PATH, so this checks both real install locations quiver.desktop's
      own Windows bundler produces (`cargo tauri build --bundles msi,nsis`,
      see docs/release-checklist.md -- both are built and published for every
      release, not just one): the NSIS setup.exe, whose installMode defaults
      to currentUser and puts the executable under %LOCALAPPDATA%\Quiver, and
      the MSI (WiX) installer, whose default scope is per-machine, under
      %ProgramFiles%\Quiver.

      Quoted paths are safe to write plainly now, exactly as in a real
      cmd.exe prompt: quiver.core's Windows step spawner sets
      SysProcAttr.CmdLine explicitly and wraps the whole command in one quote
      pair that cmd's own quote-stripping rule removes, so a quoted path
      round-trips untouched. This used to be a quote-free command for exactly
      that reason -- Go's default escaping fought cmd.exe's own re-parsing of
      its /C argument and mangled any quoted path -- which meant a path
      containing a space (any %ProgramFiles% install, or %LOCALAPPDATA% on an
      account with a space in the username) could never be expressed. That
      workaround is gone now that the underlying engine bug is fixed; both
      locations are checked directly instead of requiring an override.
    default: 'if exist "%LOCALAPPDATA%\Quiver\quiverdesktop.exe" (exit 0) else if exist "%ProgramFiles%\Quiver\quiverdesktop.exe" (exit 0) else (exit 1)'

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
          # quiver.core fixed step-field glob resolution: selector.go's
          # resolveOverrideable now runs for step fields too (previously only
          # exports: got it, via step.Overrideable.Resolve's plain exact map
          # lookup), so a "windows/*" key here now correctly resolves on both
          # windows/amd64 and windows/arm64 -- ranked exact (3) > glob (2) >
          # bare "*" (1), same specificity rule as target selection. Darwin
          # keeps the exact darwin/amd64 + darwin/arm64 keys established by
          # Task 3.1: exact was, and still is, equally correct there, and
          # this fix isn't touching that convention. The `default` is
          # therefore Linux's, not a shared Unix one.
          command:
            default: "${QUIVER_DESKTOP_DETECT_COMMAND}"
            "darwin/amd64": "${QUIVER_DESKTOP_DETECT_COMMAND_DARWIN}"
            "darwin/arm64": "${QUIVER_DESKTOP_DETECT_COMMAND_DARWIN}"
            "windows/*": "${QUIVER_DESKTOP_DETECT_COMMAND_WINDOWS}"
          timeout: "10s"
          exit_on_failure: false
```

# Quiver

<p align="center">
  <img src="https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/.github/quiver.svg" alt="Quiver" width="450" />
  <br/>
  <em>The window you're looking through right now.</em>
</p>

This is Quiver itself: the desktop app you're using to browse this catalog, showing up in it like any other application. It's listed here so it can report its own version next to everyone else's, the same way it tracks updates for anything else you've installed.

If you're reading this inside the app, there's nothing to do: you already have it. If you're not, this is how you'd get it. Quiver installs, updates and removes this window exactly the way it handles everything else in your library.

```arrow
schema: "arrow@v0"

metadata:
  name: "Quiver Desktop"
  description: "The Quiver desktop application."
  version: "0.1"
  license: "GPL-3.0"
  url: "https://github.com/rabbytesoftware/quiver.desktop"
  media:
    icon: "https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/docs/quiver-icon.svg"
    banner: "https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/docs/quiver-banner.svg"
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
      Quiver Desktop install. Two shapes count, because two shapes exist: the
      .deb puts the main binary on PATH as /usr/bin/quiverdesktop, and the
      install lifecycle below puts the AppImage at a fixed path of its own (see
      QUIVER_DESKTOP_APPIMAGE_PATH). The AppImage used to be skipped here on
      the grounds that a portable single file has no conventional location to
      check. That held until this manifest started placing one, which is
      what defines that location. install.sh at the repo root places it in the same
      spot for the same reason, so a script install and an arrow install are
      indistinguishable to this probe.

      The path is written out in full rather than as
      ${QUIVER_DESKTOP_APPIMAGE_PATH}: variable expansion is a single pass over
      the step's own field, so a ${...} sitting inside another variable's value
      is never expanded again. It would reach sh verbatim, expand to empty as
      an unset environment variable, and turn this into `test -x ""`: a probe
      that answers "not installed" on a machine where it is.
    default: 'command -v quiverdesktop >/dev/null 2>&1 || test -x "${XDG_DATA_HOME:-$HOME/.local/share}/Quiver/Quiver.AppImage"'
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
  - name: "QUIVER_DESKTOP_APPIMAGE_PATH"
    description: >-
      Where the Linux install puts the AppImage, and therefore where update
      replaces it, uninstall deletes it, and the preinstalled probe looks for
      it. The AppImage is the Linux asset this manifest installs, not the
      .deb, which only installs on Debian-family distros and needs root, while
      an AppImage runs the same everywhere and needs neither.

      The default is XDG's data home with its standard fallback, spelled the
      way the shell spells it. Quiver leaves ${XDG_DATA_HOME:-...} alone (a
      brace body that is not a name it resolved is passed through verbatim),
      so sh is what expands it, and a machine that sets XDG_DATA_HOME is
      honoured without this manifest having to read the environment itself.
    default: "${XDG_DATA_HOME:-$HOME/.local/share}/Quiver/Quiver.AppImage"
  # The release asset is not derivable from anything this manifest knows.
  # quiver.desktop's own filenames carry tauri.conf.json's static "0.1.0",
  # which never tracks the git tag a release is cut from, so neither ${REF}
  # nor any template built from it names a real file. The caller resolves the
  # asset (GitHub's releases API, filtered by extension, exactly as install.sh
  # does) and hands both values in. Same contract, same names, as
  # quiver.core's own self-manifest, so one resolver serves both self-arrows.
  #
  # Both are declared without a default, which the engine reads as "the caller
  # must name this on every execution that reads it" -- scoped to the steps the
  # method being run will actually expand, so uninstall, which expands only a
  # defaulted path, needs neither and passes nothing. A previous execution's
  # answer is not inherited either: layer 5 deliberately does not carry a
  # no-default variable forward, so a bare re-update fails by name rather than
  # quietly re-fetching the asset from the update before it.
  #
  # Inside the app, the caller is quiver.desktop itself: clicking Update (or
  # Install) on Quiver's own tile resolves the asset through
  # src-tauri/src/release/mod.rs first and sends both values with the request.
  # The checksum is read from the releases API's own per-asset digest, falling
  # back to a published checksum manifest; if a release publishes neither, the
  # app says so and does not start the update, because a fetch step cannot be
  # told to skip verification and would fail after the running app had already
  # been killed.
  - name: "QUIVER_RELEASE_ASSET_URL"
    description: "Download URL for the resolved quiver.desktop release asset for this platform: the .AppImage on Linux, the .dmg on macOS, the NSIS *-setup.exe on Windows."
  - name: "QUIVER_RELEASE_CHECKSUM"
    description: "SHA-256 of that asset, as bare lowercase hex with no algorithm prefix, for fetch-step verification. An empty value is refused rather than treated as 'unverified'."

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

      # One download, one placement. The download is identical on every
      # platform because the caller already resolved a single per-platform
      # URL; only the filename and what to do with the file differ.
      install:
        - type: fetch
          title: "Download the Quiver Desktop release asset"
          url: "${QUIVER_RELEASE_ASSET_URL}"
          to:
            default: "./Quiver.AppImage"
            "darwin/*": "./Quiver.dmg"
            "windows/*": "./Quiver-setup.exe"
          checksum: "${QUIVER_RELEASE_CHECKSUM}"
          timeout: "10m"
          exit_on_failure: true
        - type: run
          title: "Install Quiver Desktop"
          # Every $NAME here is the shell's own. Writing one as ${NAME} would
          # be read as a Quiver reference and rejected at parse time by the
          # variable_refs rule, since no such Quiver variable exists.
          command:
            default: >-
              mkdir -p "$(dirname "${QUIVER_DESKTOP_APPIMAGE_PATH}")" &&
              chmod +x ./Quiver.AppImage &&
              mv -f ./Quiver.AppImage "${QUIVER_DESKTOP_APPIMAGE_PATH}"
            # hdiutil picks its own mount point when not told otherwise, and
            # names it after the volume with a counter suffix if /Volumes/Quiver
            # is taken. That is unpredictable, so it is pinned to a path under
            # this arrow's own workdir instead of parsed back out of hdiutil's
            # output. The trap is what makes that safe: a copy that fails still
            # unmounts, instead of leaving the image attached until reboot.
            # ditto rather than cp -R because it is the copy that preserves a
            # bundle's extended attributes, and so its code signature.
            "darwin/*": |-
              set -e
              MOUNT="${WORKDIR}/dmg-mount"
              hdiutil detach "$MOUNT" -quiet -force >/dev/null 2>&1 || true
              rm -rf "$MOUNT"
              mkdir -p "$MOUNT"
              hdiutil attach ./Quiver.dmg -mountpoint "$MOUNT" -nobrowse -quiet
              trap 'hdiutil detach "$MOUNT" -quiet -force >/dev/null 2>&1 || true' EXIT
              DEST=/Applications
              [ -w "$DEST" ] || DEST="$HOME/Applications"
              mkdir -p "$DEST"
              rm -rf "$DEST/Quiver.app"
              ditto "$MOUNT/Quiver.app" "$DEST/Quiver.app"
            # /S is NSIS's silent switch. The NSIS installer is the one this
            # manifest fetches because its installMode defaults to currentUser
            # (RequestExecutionLevel user, $INSTDIR = $LOCALAPPDATA\Quiver), so
            # it needs no elevation; the .msi published alongside it is
            # per-machine and would.
            "windows/*": '.\Quiver-setup.exe /S'
          timeout:
            default: "2m"
            "windows/*": "10m"
          exit_on_failure: true

      # Fetch first, then stop the app. The reverse order, which is the
      # obvious one, takes the user's app down and only then discovers the
      # download is broken, leaving them with nothing running and nothing
      # installed.
      #
      # The manifest format has no way to share these steps with install:.
      # `base:` shares whole lifecycle hooks between targets, never steps
      # between hooks of one target, and there is no anchor or template
      # mechanism in arrow@v0. The duplication below is the format, not an
      # oversight.
      update:
        - type: fetch
          title: "Download the new Quiver Desktop release asset"
          url: "${QUIVER_RELEASE_ASSET_URL}"
          to:
            default: "./Quiver.AppImage"
            "darwin/*": "./Quiver.dmg"
            "windows/*": "./Quiver-setup.exe"
          checksum: "${QUIVER_RELEASE_CHECKSUM}"
          timeout: "10m"
          exit_on_failure: true
        - type: run
          title: "Stop the running Quiver Desktop"
          # SIGTERM, deliberately, and NOT a graceful quit. An AppleScript
          # `quit app "Quiver"` (or any route through Tauri's own quit path)
          # reaches RunEvent::ExitRequested in src-tauri/src/lib.rs, which
          # shuts the ConnectionManager down, which reaps the quiver.core the
          # app spawned as its sidecar, and on a self-update that daemon is
          # the process running this very step. A default-signal pkill has no
          # such handler to reach: the app dies, the daemon it started is
          # reparented and lives on to finish the update.
          #
          # Nothing running is the normal case when Quiver.core drives this
          # headlessly, and pkill reports it as exit 1, so failure here is not
          # a failure of the update.
          command:
            default: "pkill -x quiverdesktop"
            "windows/*": "taskkill /IM quiverdesktop.exe /F"
          timeout: "30s"
          exit_on_failure: false
        - type: run
          title: "Install the new Quiver Desktop"
          command:
            default: >-
              mkdir -p "$(dirname "${QUIVER_DESKTOP_APPIMAGE_PATH}")" &&
              chmod +x ./Quiver.AppImage &&
              mv -f ./Quiver.AppImage "${QUIVER_DESKTOP_APPIMAGE_PATH}"
            "darwin/*": |-
              set -e
              MOUNT="${WORKDIR}/dmg-mount"
              hdiutil detach "$MOUNT" -quiet -force >/dev/null 2>&1 || true
              rm -rf "$MOUNT"
              mkdir -p "$MOUNT"
              hdiutil attach ./Quiver.dmg -mountpoint "$MOUNT" -nobrowse -quiet
              trap 'hdiutil detach "$MOUNT" -quiet -force >/dev/null 2>&1 || true' EXIT
              DEST=/Applications
              [ -w "$DEST" ] || DEST="$HOME/Applications"
              mkdir -p "$DEST"
              rm -rf "$DEST/Quiver.app"
              ditto "$MOUNT/Quiver.app" "$DEST/Quiver.app"
            "windows/*": '.\Quiver-setup.exe /S'
          timeout:
            default: "2m"
            "windows/*": "10m"
          exit_on_failure: true
        - type: run
          title: "Relaunch Quiver Desktop"
          # Detached on every platform, because a run step waits for what it
          # starts. On Linux that means redirecting all three streams as well
          # as backgrounding: a child that keeps the step's inherited stdout
          # pipe open holds the spawner's readers at EOF-never, and the step
          # sits there until its timeout even though the shell itself exited.
          # `open` and `start` sidestep that by handing the launch to
          # LaunchServices and to a new console respectively, so neither
          # inherits this step's pipes at all.
          command:
            default: 'nohup "${QUIVER_DESKTOP_APPIMAGE_PATH}" >/dev/null 2>&1 </dev/null &'
            "darwin/*": 'APP=/Applications/Quiver.app; [ -d "$APP" ] || APP="$HOME/Applications/Quiver.app"; open -a "$APP"'
            "windows/*": 'start "" "%LOCALAPPDATA%\Quiver\quiverdesktop.exe"'
          timeout: "30s"
          exit_on_failure: false

      uninstall:
        - type: run
          title: "Stop the running Quiver Desktop"
          command:
            default: "pkill -x quiverdesktop"
            "windows/*": "taskkill /IM quiverdesktop.exe /F"
          timeout: "30s"
          exit_on_failure: false
        - type: run
          title: "Remove Quiver Desktop"
          # Each platform removes what its own install step created, and is
          # silent about what was never there. The macOS form tests
          # /Applications before deleting from it so that "no bundle there" and
          # "a bundle there this user may not delete" stay distinguishable: the
          # first is success, the second is the failure it actually is.
          #
          # Windows runs the uninstaller NSIS wrote next to the executable
          # ($INSTDIR\uninstall.exe, per Tauri's installer template). It
          # returns as soon as it has re-executed itself out of %TEMP% (that
          # is how an NSIS uninstaller deletes its own directory), so this
          # step reports the uninstall started, not finished. A per-machine MSI
          # install is deliberately left alone: this manifest never installs
          # one, and removing it would need elevation.
          command:
            default: 'rm -f "${QUIVER_DESKTOP_APPIMAGE_PATH}"'
            "darwin/*": 'rm -rf "$HOME/Applications/Quiver.app"; [ ! -d /Applications/Quiver.app ] || rm -rf /Applications/Quiver.app'
            "windows/*": 'if exist "%LOCALAPPDATA%\Quiver\uninstall.exe" ("%LOCALAPPDATA%\Quiver\uninstall.exe" /S) else (exit 0)'
          timeout: "2m"
          exit_on_failure: true
```

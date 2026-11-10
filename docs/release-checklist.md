# Release checklist

Manual checklist for shipping a native (GPUI) `quiver.desktop` release that bundles a quiver.core sidecar. What
cannot be automated is called out as such: macOS notarization needs Apple credentials, Windows code signing needs
a certificate, and the update-from-the-previous-release check needs a person (or the box scenario) to watch the
real app replace itself. `make gate` runs the PR checks locally. Everything else has an xtask subcommand or a Makefile target: use it, do not hand-run the
equivalent commands.

Run every step in order.

## 0. Prerequisites

- Push access to `rabbytesoftware/quiver.core` and `rabbytesoftware/quiver.desktop`, merge rights on
  `master`/`develop`.
- `gh` authenticated (`gh auth status`): `cargo xtask resolve-core-version`, `extract-core-version` and
  `fetch-sidecar` shell out to it.
- A Mac with Xcode command-line tools for the notarization check; a Windows machine for the signing and install
  checks; Docker (or the `E2E (pre-release)` job) for the box.
- The pipeline that builds a release is `.github/workflows/release-native.yml`, called by `nightly.yml`,
  `prerelease.yml` and `stable-release.yml` (see step 3).

## 1. Cut a `stable-*` tag on quiver.core and confirm its release + checksum are published

`quiver.core` releases are never tagged by hand. `stable-*` tags
(`stable-26.5`, `stable-26.5.1`, …) are cut automatically by
`.github/workflows/stable-release.yml` when a PR from a `beta/<series>` or
`hotfix/<name>` branch merges into `master`:

1. Open (or confirm the existence of) a PR from `beta/<series>` (e.g.
   `beta/26.6`) or `hotfix/<name>` into `master` in `quiver.core`, containing
   everything meant to ship.
2. Merge it. The `Stable Release` workflow's `prepare` job computes the next
   tag (bumping the patch on the series if one already exists, per the logic
   in that job — read it before assuming a specific number), its `build` job
   builds all five platform binaries via `build-assets.yml`
   (`quiver-linux-amd64`, `quiver-linux-arm64`, `quiver-darwin-amd64`,
   `quiver-darwin-arm64`, `quiver-windows-amd64.exe`) and generates
   `checksums.txt` via `sha256sum ./* > checksums.txt`, and its `publish` job
   pushes the tag and runs `gh release create` with all six files attached.
3. Confirm the release and its assets exist:
   ```bash
   gh release view <stable-tag> --repo rabbytesoftware/quiver.core --json assets --jq '.assets[].name'
   ```
   Expect exactly the five binaries above plus `checksums.txt`.
4. Confirm the checksums are real and match, not just present:
   ```bash
   gh release download <stable-tag> --repo rabbytesoftware/quiver.core \
     --pattern "quiver-*" --pattern "checksums.txt" --dir /tmp/quiver-release-check
   (cd /tmp/quiver-release-check && sha256sum -c checksums.txt)
   ```
   Every line must print `OK`. This is the same checksum verification
   `quiver.core`'s own fetch step performs on every arrow download
   (`internal/engine/wizard/internal/step/download/handler.go`,
   `verifyChecksum` / `ErrChecksumMismatch`) — confirming it by hand here
   catches a corrupted upload before any Desktop build or self-update fetch
   ever depends on it.


## 2. Confirm quiver.desktop's stable channel resolves to the intended tag

`installer/core-channels.json` maps each build channel to the quiver.core channel it tracks (`stable`, `beta`,
`nightly`), never a pinned version. `cargo xtask resolve-core-version <channel>` resolves the channel against
quiver.core's published releases (`gh release list`), picking the highest tag with quiver.core's own ranking
(calendar `26.5.1` and date `2026-09-27.1` series, beta counts, the rolling `nightly-latest`); its tests encode
the ranking. A stable release always bundles whatever quiver.core currently calls newest stable; the smoke tests
below catch an incompatible combination, not configuration.

```bash
rm -f core-version.local                 # a local pin would mask the real resolution
cargo xtask resolve-core-version stable   # prints the tag, e.g. stable-26.5.1
make fetch-sidecar                        # downloads it to target/sidecar/<triple>/quiver[.exe]
```

The printed tag must be the one cut in step 1. If quiver.core has several `stable-*` releases and the newest is
not the one intended, that is a sequencing problem to fix before continuing. A hotfix does not resolve a channel:
it pins to what the latest stable desktop release recorded (`cargo xtask extract-core-version stable-<tag>`
reads its "Built against quiver.core `...`" line).

### How each build files its own catalog row

On every connection to the LOCAL daemon the app announces itself (`quiver-core-client::self_announce`). The row is
`github.com/rabbytesoftware/quiver.desktop@<channel>`, chosen from what the pipeline compiled into the build
(`crates/quiver-release/build.rs`, `crates/quiver-app/src/services/release_vars.rs`):

| Workflow input | `release-channel` (env `QUIVER_DESKTOP_BUILD_CHANNEL`) | `release-tag` (env `QUIVER_DESKTOP_RELEASE_TAG`) | Row | Declares the running build (`/adopt`) |
|---|---|---|---|---|
| stable | `stable` | the `stable-*` tag | `@stable` | yes |
| beta | `beta` | none | `@beta` | no |
| hotfix | `hotfix` | none | `@hotfix` | no |
| nightly | `nightly-rolling` | none | `@nightly-rolling` | no |
| PR, local or e2e build | none | none | the repository's default channel | no |

`build-label` (`QUIVER_DESKTOP_STAMP_LABEL`) is display only. A channel the repository does not publish yet falls
back to the default one. **Known limit: only stable builds declare what is running**; a beta, hotfix or nightly
build's row resolves to whatever its channel pointed at when first registered, so its first version check can
offer (or miss) a build that does not match the installed one.

## 3. Build the release bundles

Release bundles come from `.github/workflows/release-native.yml`, the one shared pipeline. The three
branch-driven workflows keep only their trigger and tag derivation and call it with `channel` and `tag`:
`nightly.yml` (push to `develop`, tag `nightly-rolling`, replaced each run), `prerelease.yml` (push to
`beta/**` / `hotfix/**`, tags `beta-<series>[-n]` / `hotfix-<v>[-n]`, then comments on the PR) and
`stable-release.yml` (merged PR `beta/*` or `hotfix/*` into `master`, tag `stable-<series>[.patch]`, then
`backport.yml`). It can also be run by hand (`workflow_dispatch`, `publish: false` uploads workflow artifacts
only). Its `prepare` job resolves the quiver.core tag (`cargo xtask resolve-core-version` for beta/stable/nightly;
`extract-core-version` of the latest stable for a hotfix, also when a hotfix is promoted to stable), checks that
release exists, and for stable creates and pushes the tag. Its `build` job runs `.github/actions/build-native` on ubuntu, macos-14 and windows: fetch the sidecar, `cargo build
--release -p quiver-app` (never `--features e2e-bridge`), `cargo xtask package`, then
`cargo xtask check-release-artifact` on the binaries. Its `publish` job runs `cargo xtask checksums` and
`cargo xtask check-release-assets` (below) before `gh release create`.

To build the same bundle locally: `make package` (macOS builds the universal dmg and needs both sidecars).
Outputs land in `target/bundle/`:

| OS | Asset (what `quiver-release` selects by suffix) | Also |
|---|---|---|
| Linux | `Quiver_<v>_amd64.AppImage` | `quiver-desktop_<v>_amd64.deb`; a `.tar.gz` only if `appimagetool` is missing (not usable by the updater) |
| macOS | `Quiver_<v>_universal.dmg` containing `Quiver.app` | |
| Windows | `Quiver_<v>_x64-setup.exe` (NSIS, per-user, `%LOCALAPPDATA%\Quiver`) | |

Every bundle installs `quiverdesktop[.exe]` and the quiver.core sidecar as
`quiver[.exe]` next to each other (decision-log: "Installed binary names"). `<v>` is the numeric bundle
version (`0.1.0`), which does not track the git tag: the updater picks assets by suffix and architecture alias.

`cargo xtask check-release-assets release/ --arrow ARROW.md` is the pre-publish gate: one asset for each of
`.AppImage`, `.dmg`, `-setup.exe`; a `checksums.txt` (sha256sum format, written by `cargo xtask checksums`) that
lists each; and an `ARROW.md` that still declares `QUIVER_RELEASE_ASSET_URL` and `QUIVER_RELEASE_CHECKSUM`.
`ARROW.md` is not uploaded: quiver.core reads it from the repository at the announced ref.

## 4. macOS signing and notarization

`cargo xtask package macos` signs, hardened-runtime-enables, notarizes and staples when the Apple secrets are
present, and otherwise ad-hoc signs and says so in the log (it fails open to an unsigned DMG, so verify):

- `APPLE_CERTIFICATE` (base64 `.p12`, imported by the `build-native` action into a temporary keychain),
  `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` ("Developer ID Application: Name (TEAMID)", the switch
  that turns signing on), `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID`. With the identity but not
  the last three it signs and skips notarization, which is not enough for this step.
- Entitlements are `installer/macos/Entitlements.plist` (JIT and unsigned executable memory: needed by the
  WKWebView of the arrow-app helper, which is signed with the same entitlements).

Verify the artifact, not the log, on a Mac that never saw this app:

```bash
xcrun stapler validate Quiver_*.dmg
hdiutil attach Quiver_*.dmg && spctl -a -vvv -t install /Volumes/Quiver/Quiver.app
lipo -info /Volumes/Quiver/Quiver.app/Contents/MacOS/quiverdesktop   # x86_64 arm64
```

`spctl` must say `accepted` and `source=Notarized Developer ID`. **Untested from the development container**: the
macOS path (`lipo`, `hdiutil`, `codesign`, `notarytool`) has never run; the first CI run is its first test.

## 5. Windows installer and code signing

`cargo xtask package windows` compiles `installer/windows/quiver.nsi` with `makensis` (installer hooks reused from
the earlier Tauri installer: `windows-installer-hooks.nsh` installs the VC++ runtime when missing; `QUIVER_VCREDIST_PATH`
must point at `vc_redist.x64.exe`, which the action downloads and checks for a Microsoft signature). The
installer is per-user and has no elevation of its own; **a UAC prompt appears only when the VC++ runtime is
missing** (the `runas` step). There is still no Authenticode signing in the pipeline:

```powershell
signtool sign /fd SHA256 /f <cert>.pfx /p <password> /tr http://timestamp.digicert.com /td SHA256 Quiver_*-setup.exe
signtool verify /pa Quiver_*-setup.exe
```

(then re-run `cargo xtask checksums` on the folder: signing changes the hash). Manual checks on a clean Windows:

- Run the installer, then `Quiver_*-setup.exe /S` over it (the silent switch `ARROW.md` uses to update); the app
  lives at `%LOCALAPPDATA%\Quiver\quiverdesktop.exe`; `%LOCALAPPDATA%\Quiver\uninstall.exe /S` removes it.
- On a machine without the VC++ runtime a UAC prompt appears and names the publisher (not "Unknown Publisher" once
  signed); no human can click it in CI.
- The app launches and reaches the Home view.

**Untested from the development container**: the NSIS script and the whole Windows path.

## 6. Run the E2E box against the release-shaped build

`make test-e2e-box` (CI: `E2E (pre-release)` on every `stable-*` tag and by hand, the `E2E box` job of `ci.yml` on PRs; both upload `docker/e2e/results/` as `e2e-box-results`) builds the app with
the bridge feature, builds real quiver.core versions and runs the Rust scenarios (`docs/e2e.md`). Read the
logs in `docker/e2e/results/native/`, not just the checkmark: `SUMMARY.txt` lists each test and `PASS`/`FAIL`.
The box tests a bridge-enabled build of the same source, not the release binary: the release binary is covered
by `check-release-artifact` plus step 7.

## 7. Update from the previous release

The updater path (the app asks quiver.core to run `ARROW.md`'s update lifecycle with the asset URL and checksum the
app resolved) has two halves.

**Automated, file level**: `cargo xtask verify-update --from <previous Quiver_*.AppImage> --to <new Quiver_*.AppImage>`
(Linux; also takes the `.tar.gz` fallback, minus the placement check). It unpacks both, asserts the expected
layout (`quiverdesktop`, sidecar `quiver`), asserts neither carries the e2e bridge, that
the two builds differ, and replays `ARROW.md`'s Linux update step in a scratch `XDG_DATA_HOME` (replace
`Quiver/Quiver.AppImage`, keep it executable) checking the result is byte-for-byte the new bundle. It does NOT
run the app. It is runnable in the box or in CI after downloading the previous release's asset:

```bash
gh release download <previous-stable-tag> --repo rabbytesoftware/quiver.desktop --pattern '*.AppImage' --dir prev
cargo xtask verify-update --from prev/Quiver_*.AppImage --to target/bundle/Quiver_*.AppImage
```

**Not automated, do by hand** (the in-app half; the box scenario `self-update-while-running` and the desktop
update test cover it with stand-in releases and fake builds, never with the real previous release):

1. Install the PREVIOUS published release the way a user does (DMG, setup.exe, AppImage), launch it, note its
   Settings/About build label.
2. Publish the new release as a prerelease on a scratch tag if it is not yet public, or point a test machine's
   upstream at it.
3. In the app open Quiver's own tile (Library), press Update. Expected: the app resolves the asset (the page
   digest, else `checksums.txt`; with neither it refuses before touching anything), quiver.core downloads and
   verifies it, stops the app, runs the installer/replaces the AppImage, and `ARROW.md`'s start step relaunches it.
4. After relaunch: About/Settings shows the new label, the Library shows the same installed arrows (nothing was
   lost), the sidecar is the newer-or-same quiver.core, and the previous process is gone (`pgrep quiverdesktop`
   shows one).
5. Repeat once on each OS, and once from the oldest release you still support.

## 8. Manual smoke test: a stale desktop gets told it is outdated

1. Take an install whose self-installed quiver.core is older than the one just published (pin
   `core-version.local` to an older `stable-*`, `make fetch-sidecar`, `make dev`).
2. Launch it to the normal UI, open Library.
3. Wait for quiver.core's periodic version check (within a minute of daemon start, then every
   `arrows.version_check_interval`, 6h), or open the quiver.core self-arrow's detail page to trigger it
   (throttled by `version_check_ttl`).
4. The quiver.core tile must show the "Update available" badge, the same one any outdated arrow gets. There is no
   self-arrow-specific UI.
5. No badge: first rule out `version_check_ttl` throttling and propagation delay after step 1's publish.

## 9. Manual smoke test: a supervised process survives a real self-update

1. With a build of the new release, install and start an arrow whose runtime quiver.core supervises as a
   long-running process.
2. Note its PID from `GET /v0/runtime/<ns>`.
3. Trigger quiver.core's own self-update (update its self-arrow to a newer version).
4. Once quiver.core is back (same process image on unix, a new PID on Windows by design) re-read the PID: it must
   be unchanged and the process still `running`. A changed PID or dead process is a release blocker.

## 10. Ship

Only when every step is confirmed: release assets and checksums verified (1), the desktop build bundles the
intended core tag (2), `check-release-assets` and `check-release-artifact` pass (3), the macOS build is
notarized (4), the Windows installer is signed and installs/updates silently (5), the box was run and its output
read (6), the update from the previous release was verified by `verify-update` and by hand (7), and the smoke
tests pass (8, 9): publish the release (draft first if signing artifacts were re-done by hand, after
re-running `cargo xtask checksums`).

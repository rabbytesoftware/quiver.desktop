# Release checklist

This is the manual checklist for shipping a `quiver.desktop` release that
bundles a new `quiver.core` sidecar. It exists because the one part of this
pipeline that genuinely cannot be automated — macOS notarization and Windows
code-signing plus the interactive UAC elevation prompt a real Windows install
requires — needs real signing credentials and, on Windows, a human physically
confirming a native OS dialog. Neither CI nor the `tauri-driver` E2E harness
(`e2e/`) can drive that headlessly: `tauri-driver`'s WebDriver session
controls the app's own WebView, not OS-native modal dialogs outside it.

Everything else in this list either has a script/Makefile target of its own
(follow it, don't hand-run the equivalent commands) or is a real, currently
manual gap — called out honestly as such rather than described with invented
specifics.

Run every step in order. Do not skip ahead because an earlier step "probably"
passed.

## 0. Prerequisites

- Push access to both `rabbytesoftware/quiver.core` and
  `rabbytesoftware/quiver.desktop`, and merge rights on `master`/`develop` as
  the branch policy requires.
- `gh` CLI authenticated (`gh auth status`) — both `scripts/resolve-core-version.mjs`
  and `make fetch-sidecar`/`make pr-checks` shell out to it.
- A macOS machine with Xcode command-line tools, for the notarization steps.
  A Windows machine (or VM) for the code-signing and interactive-install
  steps — the elevation prompt in step 4 cannot be confirmed remotely from a
  Mac or Linux box.
- A Linux machine (or the `E2E (pre-release)` GitHub Actions job) for step 5 —
  `tauri-driver` has no macOS backend (see `e2e/README.md`'s "Platform
  support" section); this is a property of `tauri-driver` itself, not this
  repo.

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

## 2. Confirm quiver.desktop's `coreVersion` constraint resolves to the intended tag

`quiver.desktop`'s `package.json` pins a semver range, not an exact tag —
currently `"quiver": { "coreVersion": "^26.5" }`. `scripts/resolve-core-version.mjs`
resolves that range against `quiver.core`'s real published `stable-*` releases
(via `gh release list --repo rabbytesoftware/quiver.core`), picking the
highest matching one.

1. If the new tag from step 1 falls outside the current range (e.g. a new
   major/minor series), update `quiver.coreVersion` in `package.json` first
   and commit that change through the normal PR process.
2. Confirm resolution using the real Makefile invocation — `make fetch-sidecar`
   (the same target the dev loop and CI both use to pull the sidecar) prints
   both the constraint and what it resolved to:
   ```bash
   make fetch-sidecar
   ```
   Expect a line like `📥 Fetching quiver.core sidecar (stable-26.5.1, matched ^26.5) for <target-triple>...`
   followed by `✅ Sidecar ready: src-tauri/binaries/quiver-<target-triple>`.
   If it fails with `no stable release found satisfying "..."`, the tag from
   step 1 either isn't published yet or genuinely doesn't satisfy the range —
   fix the constraint or wait, don't proceed.
3. `make pr-checks`'s own "Step 1/6: CORE_VERSION Validation" performs the
   same resolution plus a remote existence check
   (`gh release view "$RESOLVED_VERSION" ...`) — this already runs in CI on
   every PR, so a merged `quiver.desktop` PR has implicitly re-confirmed this;
   step 2 above is the pre-release-specific confirmation against the tag just
   cut.

## 3. macOS notarization

Unlike Windows (step 4), macOS signing here is **not** a from-scratch
manual process — `scripts/build-macos-dmg.sh` (invoked by
`.github/actions/build-tauri`'s macOS build step, which
`.github/workflows/stable-release.yml`'s `build` job runs on `macos-26`) is
already wired to sign, hardened-runtime-enable, notarize, and staple the
universal DMG automatically via Tauri's own bundler, *if* six Apple secrets
are present in the environment: `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
`APPLE_PASSWORD`, `APPLE_TEAM_ID`. The script's own comment is explicit:
"Signing + notarization are OPT-IN and driven entirely by environment
variables" — when they're unset (as they currently are, since this effort has
never configured them), it builds an ad-hoc/unsigned DMG instead, silently.
So the real manual work here is (a) a one-time setup of those secrets if they
aren't already configured, and (b) verifying, for *this* release, that
signing actually engaged rather than silently falling back to unsigned.

1. In GitHub → `quiver.desktop` repo → Settings → Secrets and variables →
   Actions, confirm all six secrets above are present. If this is the first
   time setting this up:
   - Obtain a **Developer ID Application** certificate from an active Apple
     Developer Program account (whoever holds Rabbyte Software's membership —
     this repo has no existing record of one), export it as a base64-encoded
     `.p12` for `APPLE_CERTIFICATE`, and set `APPLE_CERTIFICATE_PASSWORD` to
     its export password.
   - Set `APPLE_SIGNING_IDENTITY` to the exact identity string, e.g.
     `"Developer ID Application: <Org Name> (<Team ID>)"` — its presence is
     literally the switch `build-macos-dmg.sh` checks to decide signed vs.
     ad-hoc.
   - Set `APPLE_ID` / `APPLE_PASSWORD` (an **app-specific** password for that
     Apple ID, not its login password) / `APPLE_TEAM_ID` for notarization —
     if these three are left unset, the script's own comment notes Tauri
     signs but skips notarization, which is not sufficient for this step.
   - `src-tauri/Entitlements.plist` already exists in the repo and is wired
     in via `entitlements: "Entitlements.plist"` once signing is on — no
     separate entitlements setup is needed.
2. Merge the `quiver.desktop` release PR (`beta/<series>`/`hotfix/<name>` →
   `master`, same convention as step 1) so `stable-release.yml`'s macOS build
   job runs with those secrets in scope, producing
   `src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg` already
   signed, notarized, and stapled by the time the job uploads
   `macos-artifacts`.
   - To reproduce or debug the same build locally on a Mac instead: export
     the six variables above, then run `bash scripts/build-macos-dmg.sh`
     directly (it resolves paths relative to itself, so it can be run from
     the repo root).
3. Don't trust the job log alone — verify the artifact itself. Download
   `macos-artifacts` from the run (or use the local build's output) and run:
   ```bash
   xcrun stapler validate Quiver_*.dmg
   spctl -a -vvv -t install /Volumes/Quiver/Quiver.app   # after mounting the DMG
   ```
   `spctl` must report `accepted` and `source=Notarized Developer ID`. Test
   this on a Mac that has never seen a build of this app before, or with
   Gatekeeper's cache cleared — a machine that already ran an unsigned build
   of the same app can give a false pass from its local cache.
4. If `spctl` reports anything else (`rejected`, or `source=Unnotarized
   Developer ID`), the secrets in step 1 are missing or wrong — re-check them
   before re-running the release build, since `build-macos-dmg.sh` fails open
   to an unsigned build rather than erroring loudly.

## 4. Windows code-signing and the interactive elevation prompt

Unlike macOS, there is genuinely no signing infrastructure for Windows
anywhere in this repo — no `certificateThumbprint`, `digestAlgorithm`, or
`signCommand` under `bundle.windows` in `tauri.conf.json` (verified), and
`.github/actions/build-tauri`'s Windows step
(`cargo tauri build --bundles msi,nsis --config '{"bundle":{"macOS":{"files":{}}}}'`,
run on `windows-latest` by `stable-release.yml`'s `build` job) has no
counterpart to `scripts/build-macos-dmg.sh` — it produces
`src-tauri/target/release/bundle/{msi,nsis}/*` completely unsigned, uploaded
as-is under the `windows-artifacts` artifact. The steps below are standard
Authenticode signing steps applied by hand after that CI build — obtain the
actual code-signing certificate from wherever Rabbyte Software's real process
for that lives before this step is meaningful; there's nothing in this repo
to point to instead.

1. Obtain a code-signing certificate (OV or EV) from a CA, as a `.pfx`/`.p12`
   or installed in the Windows certificate store.
2. Download the unsigned `windows-artifacts` from the same
   `stable-release.yml` run that step 3 already triggered by merging the
   release PR (its `build` job runs the Linux, macOS, *and* Windows builds
   in parallel), or build locally on Windows with `make build-app`. Either
   way you get `src-tauri/target/release/bundle/nsis/Quiver_*-setup.exe`
   and/or `bundle/msi/*.msi`.
3. Sign it with `signtool` by hand — there is no `bundle.windows.signCommand`/
   `certificateThumbprint` config in this repo to wire it through Tauri
   directly, so this step is a manual post-build pass:
   ```powershell
   signtool sign /fd SHA256 /f <path-to-cert>.pfx /p <cert-password> `
     /tr http://timestamp.digicert.com /td SHA256 `
     src-tauri\target\release\bundle\nsis\Quiver_*-setup.exe
   signtool verify /pa src-tauri\target\release\bundle\nsis\Quiver_*-setup.exe
   ```
   `signtool verify` must report success before continuing.
4. **Interactive elevation check — cannot be automated.** On a real Windows
   machine (not a headless CI runner), double-click the signed installer.
   Windows will show a native UAC "Do you want to allow this app to make
   changes to your device?" dialog naming the signed publisher. A human must
   click **Yes**; `tauri-driver`'s WebDriver session only controls the app's
   own WebView content once it starts, not this OS-native dialog, so this is
   the one step in the whole pipeline that has no scripted substitute.
   Confirm:
   - The publisher name in the UAC dialog matches the certificate from step 1
     (not "Unknown Publisher" — a sign that the signature didn't take).
   - The app launches after install and reaches the normal UI (confirms the
     signed binary itself runs, not just that signing succeeded).

## 5. Run the E2E harness against the release build

Run the `tauri-driver` harness (`e2e/`, `.github/workflows/e2e.yml`) against
the actual release-shaped build before shipping. It runs automatically on
every `stable-*` tag push, or on demand via `workflow_dispatch`; it can also
be run locally on Linux (`e2e/README.md`'s "Running locally" section) for a
faster pre-tag check.

1. `.github/workflows/e2e.yml` lives in **`quiver.desktop`** and triggers on
   a `stable-*` tag push **in that repo** — the tag `quiver.desktop`'s own
   `stable-release.yml` already cut as part of merging the release PR in
   steps 3–4 above. That merge is the trigger; no separate action is needed
   unless re-running on demand, in which case use
   `gh workflow run e2e.yml --repo rabbytesoftware/quiver.desktop`.
2. Read the job's own logs, not just its checkmark. **This job currently runs
   with `continue-on-error: true`**, for three real, currently-unmet
   preconditions documented directly in `.github/workflows/e2e.yml`'s own
   comment:
   - `bootstrap.spec.ts` needs `quiver.desktop`'s own `ARROW.md` reachable at
     a ref `quiver.core` can resolve on the published repo. Which ref
     `announceSelf` asks for depends on how the build under test was made,
     and both answers clear on the normal merge path: a build cut from a
     `stable-*` tag carries that tag (baked in by `src-tauri/build.rs`) and
     announces `<ns>@<tag>`, which resolves because pushing that tag is what
     triggered the run; any other build announces the namespace **refless**,
     which core reads as "the latest `stable-*` release, else the default
     branch". So for a real release this is satisfied by the tag cut in
     steps 3–4, and it is only unmet when the harness is run by
     `workflow_dispatch` from a branch that has not merged yet. (Before this
     was fixed, the announce carried `@0.1.0` — `tauri.conf.json`'s
     `version` — and demanded a git ref literally named `0.1.0`, which this
     repo's `stable-<series>[.patch]` tagging never creates; that 404 was
     permanent, not pending.)
   - `self-update-while-running.spec.ts` needs a fixture arrow a *production*
     daemon can actually resolve (`QUIVER_E2E_FIXTURE_NS`) — no such fixture
     exists yet, only the Go integration suite's in-process stub resolver.
   - `generic-outdated-badge.spec.ts` structurally cannot show drift in this
     job's own environment: `make fetch-sidecar` always fetches the
     *highest* matching stable release, so the app under test in this job is
     always exactly current — there is nothing to be outdated against. This
     is a gap in the CI job's own setup, not in the mechanism itself, which
     this effort already confirmed working end-to-end by running the
     scenario manually against a genuinely stale build; step 6 below is that
     same manual check, done again for this specific release, precisely
     because CI cannot provide it.
3. What a green run *does* prove automatically, once each precondition above
   is actually met for a given release: the self-installed Core bootstrap,
   the self-arrow registration, and (with `QUIVER_E2E_FIXTURE_NS` set) that a
   supervised process really survives Core's real self-update handover.
4. Because of the gaps above, a passing (or `continue-on-error`-masked
   failing) CI run is **not sufficient by itself** to ship on. Open the run's
   uploaded logs (`e2e-logs` artifact on failure, or the step output directly)
   and manually confirm which of the three scenarios actually ran to
   completion versus failed on a known precondition — do not treat a red run
   as "expected, ignore it" without reading which failure it actually was.
   Any failure *other than* the three listed above is a real regression and
   blocks the release.
5. While you have run logs open, confirm the **`Stable Release`** run (not
   this one) actually stamped the artifacts you are about to ship. In each of
   its three `Build (<os>)` jobs, the `Validate the release tag, when one was
   given` step must print:
   ```
   ✅ stamping this build as <stable-tag>
   ```
   If it printed `ℹ️  no release-tag given` instead, the tag never reached
   `src-tauri/build.rs`, and the shipped app will announce its own arrow
   refless: it still registers and still works, but its catalog row tracks the
   latest published release rather than the build the user installed, so it
   will show an "update available" badge (above a no-op Update action, since
   `ARROW.md` declares no `update` lifecycle) as soon as anything newer lands.
   That is a degraded release, not a broken one — decide deliberately whether
   to re-run the build with the tag wired through rather than discovering it
   from user reports later.

## 6. Manual smoke test — a stale desktop actually gets told it's outdated

This is the scenario `generic-outdated-badge.spec.ts` proves in principle but
cannot exercise for real in the CI job's own environment (step 5 above), and
the one this effort's Task 4.2 fixed and Task 5.1 re-confirmed working
end-to-end.

1. Take (or build) a `quiver.desktop` install whose self-installed
   `quiver.core` is pinned to an **older** `stable-*` tag than the one just
   published in step 1 — e.g. a build from before this release, or one built
   with `quiver.coreVersion` temporarily narrowed to the previous series.
2. Launch it and let it reach the normal UI (`waitForAppReady` in the E2E
   harness is the same condition to wait for by eye: the app window is up and
   the sidebar/library have loaded).
3. Open the **Library** page (not the sidebar arrow list — `ArrowRow` in the
   sidebar draws no status badge at all; the badge lives on `ArrowTile`,
   consumed by Home, Library, and the collection grids — see
   `src/features/sidebar/components/arrows/`).
4. Open the quiver.core self-arrow's detail page (or otherwise trigger a
   `GetDetail` read for it) to fire the passive drift check
   (`CheckVersionDrift`, throttled by `version_check_ttl`) against the now
   newer remote tag.
5. Return to the Library grid and confirm the quiver.core tile shows a badge
   with the text **"Update available"** (`arrow.state.outdated` in
   `src/lib/i18n/locales/en.ts`) and a single up-arrow icon — the exact same
   `data-slot="badge"` element any other outdated arrow's tile gets. There
   must be no self-arrow-specific UI: the bespoke `core://update_status` pipe
   was deliberately removed in Task 4.1/4.2 in favor of this generic path.
6. If the badge doesn't appear, don't assume the mechanism is broken before
   checking the mundane cause first: `version_check_ttl` throttling meaning
   the drift check hasn't re-run yet, or the remote genuinely not yet
   reflecting the new tag (propagation delay right after step 1's publish).

## 7. Manual smoke test — a running supervised process survives a real self-update

This is what `self-update-while-running.spec.ts` proves through the real app
and daemon (not the simulated restart quiver.core's own
`tests/integration/selfupdate/selfupdate_test.go` uses, which is explicit
that it never performs the real OS-level handover). Repeat it by hand against
the actual release build as a final confirmation.

1. Using a `quiver.desktop` build against the **new** release (from step 1),
   install and start some arrow whose runtime quiver.core supervises as a
   long-running process (anything that reaches `running` state and stays
   there — e.g. any arrow with a long-lived `execute` lifecycle).
2. Note its PID by querying `GET /v0/runtime/<ns>` on quiver.core's own API
   (the app talks to this same endpoint over the local Unix socket / named
   pipe), which reports the supervised process's recorded PID.
3. Trigger quiver.core's own self-update: install/update its self-arrow to a
   newer version (the same lifecycle `selfarrow`/`internal/core/selfupdate`
   drive — `Trigger.Fire` records the new binary and requests the normal
   graceful shutdown; `Trigger.Relaunch` then hands over via `syscall.Exec`
   on unix or a detached respawn on Windows, per
   `internal/core/selfupdate/selfupdate_unix.go` /
   `selfupdate_windows.go`).
4. After the handover completes and quiver.core is back up (same process
   image on unix, a new PID on Windows — by design, see the comment on
   `handOver` in `selfupdate_windows.go`), re-check the *supervised arrow's*
   PID from step 2. It must be **unchanged** and the process must still be
   responsive (still `running`, not `absent`/crashed) — proving quiver.core's
   own self-update handover did not kill or orphan the process it supervises.
5. If the PID changed or the process died, this is a release blocker — the
   crash-recovery/handover guarantee this whole effort is built on has
   regressed for real, not a harness artifact.

## 8. Ship

Only once every step above is confirmed — release assets and checksums
verified (1), the desktop build resolves the intended core tag (2), the
macOS build is signed and notarized and the Windows build is signed (3, 4),
the E2E harness has been run and its real output read rather than assumed
(5), and both manual smoke tests pass against the actual release artifacts
(6, 7) — publish the signed `quiver.desktop` release (notarized macOS
`.dmg`, signed Windows installer) through the normal distribution channel.

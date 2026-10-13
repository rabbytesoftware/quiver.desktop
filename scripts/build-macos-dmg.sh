#!/usr/bin/env bash
#
# Builds the universal macOS DMG for Quiver.
#
# Signing + notarization are OPT-IN and driven entirely by environment
# variables. When the Apple signing secrets are present, the produced bundle is
# signed with a Developer ID identity, notarized by Apple, and stapled — giving
# downloaders the benign "downloaded from the Internet" Gatekeeper prompt.
#
# When the secrets are absent (an unset GitHub secret expands to an empty
# string), this builds an ad-hoc-signed (effectively unsigned) universal DMG.
# That makes enabling signing a pure configuration change: add the secrets and
# the same workflow starts producing notarized builds.
#
# Env vars consumed here directly:
#   APPLE_SIGNING_IDENTITY   e.g. "Developer ID Application: Name (TEAMID)".
#                            Its presence is the switch that turns signing on.
#
# Env vars consumed transparently by the Tauri bundler when signing is on:
#   APPLE_CERTIFICATE            base64-encoded .p12 (Tauri imports it into a
#                                temporary keychain automatically)
#   APPLE_CERTIFICATE_PASSWORD   password for the .p12
#   APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID   notarization credentials
#                                (app-specific password). If unset, Tauri signs
#                                but skips notarization.
#
set -euo pipefail

# Resolve to src-tauri regardless of the caller's working directory.
cd "$(dirname "$0")/../src-tauri"

# --- Assemble the `bundle.macOS` config overlay -----------------------------
# Start empty; only add keys we actually need so the committed tauri.conf.json
# stays authoritative for everything else.
macos='{}'

# When the Tahoe adaptive-icon Assets.car did not compile (Xcode < 26), drop the
# Resources/Assets.car file mapping — otherwise the bundler aborts on a file
# that is not there. The app still gets the classic .icns from bundle.icon.
if [ ! -f icons/Assets.car ]; then
  echo "==> No icons/Assets.car: dropping the adaptive-icon file mapping."
  macos=$(jq -cn --argjson m "$macos" '$m + {files: {}}')
fi

# Turn on signing + hardened runtime ONLY when an identity is provided.
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "==> Developer ID identity present: building SIGNED + notarized bundle."
  macos=$(jq -cn --argjson m "$macos" --arg id "$APPLE_SIGNING_IDENTITY" '
    $m + {
      signingIdentity: $id,
      hardenedRuntime: true,
      entitlements: "Entitlements.plist"
    }')
else
  echo "==> No APPLE_SIGNING_IDENTITY: building ad-hoc (unsigned) bundle."
  # CRITICAL: the Tauri bundler decides whether to sign based on the *presence*
  # of the APPLE_CERTIFICATE env var, NOT on our signingIdentity gate above.
  # GitHub Actions exports an unset secret as a defined-but-empty string
  # ("" != unset), which is enough to make the bundler attempt
  # `security import ""` and fail with "SecKeychainItemImport: ... parameters
  # ... not valid". Actively unset every Apple var so an unconfigured run sees a
  # genuinely clean environment. (No-op when they were never exported.)
  unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY \
    APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
fi

# --- Build ------------------------------------------------------------------
args=(--target universal-apple-darwin --bundles dmg)
if [ "$macos" != '{}' ]; then
  args+=(--config "$(jq -cn --argjson m "$macos" '{bundle: {macOS: $m}}')")
fi

set -x
cargo tauri build "${args[@]}"

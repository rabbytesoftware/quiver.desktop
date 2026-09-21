#!/usr/bin/env bash
# SCENARIO 1
#
#   "Core installed headless -> auto-registers itself, allows new updates to
#    come around. Simulate a new update coming from upstream." Core must
#    actually self-update.
#
# WHAT IS REAL HERE. The daemon is a version-stamped production build of
# quiver.core running as an ordinary background process off PATH. It registers
# its own arrow with no help from this script. The "new release upstream" is a
# second, genuinely different build published to the local GitHub stand-in,
# found by quiver.core's own drift check reading a redirect, downloaded by
# quiver.core's own fetch step, and verified against a real sha256 before the
# handover. Nothing about the update is injected past the API.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin "1-core-self-update" \
	"headless core self-registers, sees an upstream release, and replaces itself with it"

# --- preconditions ---------------------------------------------------------

say "Preconditions: a clean machine, and one published quiver.core release"
reset_state
assert_daemon_count 0

publish_manifest rabbytesoftware/quiver.core "$CORE_V1" "/workspace/build/src/quiver.core/ARROW.md"
publish_release rabbytesoftware/quiver.core "$CORE_V1" "$BUILD_BIN/quiver-$CORE_V1"
mark_latest rabbytesoftware/quiver.core "$CORE_V1"

# --- act: install headless -------------------------------------------------

say "Installing quiver.core headless and starting its daemon"
start_headless_daemon "$BUILD_BIN/quiver-$CORE_V1"
DAEMON_PID_BEFORE="$(daemon_pid)"
assert_daemon_count 1
assert_eq "$CORE_V1" "$(daemon_version)" "the daemon serving the socket reports its version"

# --- assert: it registered ITSELF -----------------------------------------

say "Checking quiver.core put itself in its own catalog, unprompted"
CORE_ARROW="$CORE_NS@$CORE_V1"
assert_eq "200" "$(api_status GET "/v0/arrow/$(ns_enc "$CORE_ARROW")")" \
	"GET /v0/arrow/$CORE_ARROW"
assert_eq "Quiver Core" "$(arrow_field "$CORE_ARROW" '.data.name')" \
	"the self-registered arrow's name"
assert_eq "true" "$(arrow_field "$CORE_ARROW" '.data.user_installed')" \
	"the self-arrow is marked user_installed"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-before.json"

# Bootstrap the self-arrow's runtime aggregate to Ready. quiver.core's own
# manifest declares no install lifecycle ("there is nothing to install here:
# it's already running"), so this runs dependency resolution and nothing
# else -- "already installed by definition", made concrete. This is the exact
# bootstrap tests/integration/selfupdate/selfupdate_test.go performs, for the
# same reason: BeginUpdate refuses an aggregate that has never existed.
#
# Sent with NO variables, which is the point. quiver.core's own ARROW.md
# declares QUIVER_RELEASE_ASSET_URL and QUIVER_RELEASE_CHECKSUM without
# defaults, and ResolveVariables used to require every declared no-default
# variable on EVERY execution of the arrow rather than on the lifecycle that
# reads them -- so this call needed two placeholder values for an install that
# looks at neither. That is fixed (requireReferenced), and this call is the
# end-to-end proof: nothing supplies them, and the install still runs.
say "Bootstrapping the self-arrow's runtime state"
api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW")/install" \
	'{"variables":{}}' 202 >/dev/null
wait_for_state "$CORE_ARROW" ready 120

# --- act: a new release appears upstream -----------------------------------

say "Publishing quiver.core $CORE_V2 upstream"
publish_manifest rabbytesoftware/quiver.core "$CORE_V2" "/workspace/build/src/quiver.core/ARROW.md"
publish_release rabbytesoftware/quiver.core "$CORE_V2" "$BUILD_BIN/quiver-$CORE_V2"
git_tag rabbytesoftware/quiver.core "$CORE_V2"
mark_latest rabbytesoftware/quiver.core "$CORE_V2"

NEW_ASSET="$UPSTREAM_STATE/releases/rabbytesoftware/quiver.core/$CORE_V2/quiver-$CORE_V2"
NEW_URL="https://github.com/rabbytesoftware/quiver.core/releases/download/$CORE_V2/quiver-$CORE_V2"
NEW_SUM="$(sha256_of "$NEW_ASSET")"
info "upstream asset: $NEW_URL"
info "sha256:         $NEW_SUM"

# --- assert: core NOTICES, through its own drift check ---------------------

say "Waiting for quiver.core's own version check to notice"
# GET /v0/arrow/:ns is what triggers maybeCheckVersion; version_check_ttl is
# pinned to 1s in this run's config.yaml so repeated polls really do re-check.
found_outdated=0
for _ in $(seq 1 40); do
	arrow_detail "$CORE_ARROW" >/dev/null
	sleep 1
	if [ "$(arrow_field "$CORE_ARROW" '.data.outdated')" = "true" ]; then
		found_outdated=1
		break
	fi
done
[ "$found_outdated" = "1" ] || fail "quiver.core never marked its own arrow outdated after $CORE_V2 was published: $(arrow_detail "$CORE_ARROW" | jq -c '.data | {state,outdated,recommended_ref}')"
ok "quiver.core's own arrow reports outdated=true"
assert_eq "$CORE_V2" "$(arrow_field "$CORE_ARROW" '.data.recommended_ref')" \
	"the recommended ref the drift check found"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-outdated.json"

# --- act: run the update ---------------------------------------------------

say "Executing quiver.core's own update lifecycle"
api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW")/update" \
	"$(jq -nc --arg u "$NEW_URL" --arg c "$NEW_SUM" \
		'{variables:{QUIVER_RELEASE_ASSET_URL:$u,QUIVER_RELEASE_CHECKSUM:$c}}')" \
	202 >/dev/null
ok "POST /v0/runtime/$CORE_ARROW/update accepted (202)"

# --- assert: the process actually became the new build ---------------------

say "Waiting for the handover"
# On unix the handover is syscall.Exec: same PID, new process image. So the
# assertion that proves it is NOT "a new pid appeared" -- it is "the same
# process now answers with a different version".
became_v2=0
for _ in $(seq 1 90); do
	if [ "$(api_status GET /versions)" = "200" ] && [ "$(daemon_version)" = "$CORE_V2" ]; then
		became_v2=1
		break
	fi
	sleep 1
done
[ "$became_v2" = "1" ] || fail "the daemon never came back as $CORE_V2 (currently: $(daemon_version), daemons: $(list_core_daemons))"

assert_eq "$CORE_V2" "$(daemon_version)" "the version the live daemon now reports"
assert_daemon_count 1
DAEMON_PID_AFTER="$(daemon_pid)"
assert_eq "$DAEMON_PID_BEFORE" "$DAEMON_PID_AFTER" \
	"the pid across the handover (unix exec replaces the image, keeps the pid)"

# The promoted self-install path is what a later cold start -- a reboot, or
# quiver.desktop spawning a fresh sidecar -- will pick up.
assert_file "$HOME/.quiver/self/quiver" "the promoted self-installed binary"
# `quiver version` reports two lines when a daemon is reachable (client and
# daemon), so this asserts on the client line, which is the one that names the
# binary being run.
PROMOTED_VERSION="$("$HOME/.quiver/self/quiver" version | awk '/^client /{print $2}')"
assert_eq "$CORE_V2" "$PROMOTED_VERSION" \
	"the version of the promoted self-installed binary"
assert_eq "$(sha256_of "$BUILD_BIN/quiver-$CORE_V2")" "$(sha256_of "$HOME/.quiver/self/quiver")" \
	"the promoted binary is byte-identical to the published $CORE_V2 asset"

# --- assert: the download really happened, over the wire -------------------

say "Proving the new binary came down the real fetch path"
served="$(grep -c "\"event\": \"asset.served\".*\"tag\": \"$CORE_V2\"" "$RESULTS/upstream.log" || true)"
[ "${served:-0}" -ge 1 ] || fail "the upstream fixture never served the $CORE_V2 asset; the update cannot have fetched it"
ok "the upstream fixture served the $CORE_V2 asset $served time(s)"
grep "\"tag\": \"$CORE_V2\"" "$RESULTS/upstream.log" >"$SCENARIO_DIR/upstream-hits.log" || true

# --- assert: the catalog moved on -----------------------------------------

say "Checking the catalog after the restart"
api_body GET /v0/arrow | jq '.' >"$SCENARIO_DIR/catalog-after.json"
arrow_detail "$CORE_NS@$CORE_V2" | jq '.' >"$SCENARIO_DIR/self-arrow-after.json"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-old-after.json"

assert_eq "200" "$(api_status GET "/v0/arrow/$(ns_enc "$CORE_NS@$CORE_V2")")" \
	"the new version is registered"
assert_eq "Quiver Core" "$(arrow_field "$CORE_NS@$CORE_V2" '.data.name')" \
	"the new self-arrow row's name"

# The new process's own EnsureRegistered (Container.Start) is supposed to
# leave exactly one quiver.core row in the CATALOG: it finds the old self row
# still on record and moves it onto the new ref via UpgradeVersionSeeded,
# whose own reaction removes the old row -- a stale record left behind is
# dangerous, because BeginUpdate against it would fetch into a workdir that
# may still be the file this process is executing out of. GET on a removed
# namespace still answers 200 via GetDetail's live-preview fallback for an
# uncatalogued namespace, so the catalog listing is what has to be asserted,
# not the status code.
assert_eq "$CORE_V2" "$(catalogued_refs "$CORE_NS")" \
	"the quiver.core refs left in the catalog after the handover"

api_body GET /versions | jq '.' >"$SCENARIO_DIR/versions-after.json"
scenario_end

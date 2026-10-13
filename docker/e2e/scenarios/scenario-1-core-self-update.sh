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
# found by quiver.core's own version check reading the stand-in repository's
# tags, downloaded by quiver.core's own fetch step, and verified against a
# real sha256 before the handover. Nothing about the update is injected past
# the API.
#
# THE ROW IS THE CHANNEL. Both builds are stamped main.channel=stable (build.sh),
# so quiver.core files itself as quiver.core@stable and keeps that one row for
# its whole life: the update moves what the row has installed (resolved_ref)
# from v1 to v2 in place, it never creates a second row.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin "1-core-self-update" \
	"headless core self-registers, sees an upstream release, and replaces itself with it"

# --- preconditions ---------------------------------------------------------

say "Preconditions: a clean machine, and one published quiver.core release"
reset_state
reset_upstream
assert_daemon_count 0

publish_manifest rabbytesoftware/quiver.core "$CORE_V1" "/workspace/build/src/quiver.core/ARROW.md"
publish_core_asset "$CORE_V1"
mark_latest rabbytesoftware/quiver.core "$CORE_V1"
git_tag rabbytesoftware/quiver.core "$CORE_V1"

# --- act: install headless -------------------------------------------------

say "Installing quiver.core headless and starting its daemon"
start_headless_daemon "$BUILD_BIN/quiver-$CORE_V1"
DAEMON_PID_BEFORE="$(daemon_pid)"
assert_daemon_count 1
assert_eq "$CORE_V1" "$(daemon_version)" "the daemon serving the socket reports its version"

# --- assert: it registered ITSELF -----------------------------------------

say "Checking quiver.core put itself in its own catalog, unprompted"
CORE_ARROW="$CORE_NS@stable"
wait_for_catalogued "$CORE_ARROW" 30 "quiver.core registered itself"
assert_eq "$CORE_V1" "$(catalogued_refs_resolved "$CORE_NS")" \
	"quiver.core's catalog rows, as the build each resolved to"
assert_eq "Quiver Core" "$(arrow_field "$CORE_ARROW" '.data.name')" \
	"the self-registered arrow's name"
assert_eq "true" "$(arrow_field "$CORE_ARROW" '.data.user_installed')" \
	"the self-arrow is marked user_installed"
assert_eq "channel" "$(arrow_field "$CORE_ARROW" '.data.selector_kind')" \
	"how the self-arrow follows its selector"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-before.json"

# The boot that registered the row also settled its runtime: quiver.core is
# installed by definition, so an absent runtime is marked ready, and the
# build's own commit is what the v1 tag names, so there is nothing ahead.
say "Waiting for the self-arrow's runtime to settle"
wait_for_state "$CORE_ARROW" ready 120
assert_eq "absent" "$(arrow_field "$CORE_ARROW" '.data.available // "absent"')" \
	"what the version check found ahead of $CORE_V1 before anything was published"

# --- act: a new release appears upstream -----------------------------------

say "Publishing quiver.core $CORE_V2 upstream"
publish_manifest rabbytesoftware/quiver.core "$CORE_V2" "/workspace/build/src/quiver.core/ARROW.md"
publish_core_asset "$CORE_V2"
git_tag rabbytesoftware/quiver.core "$CORE_V2"
mark_latest rabbytesoftware/quiver.core "$CORE_V2"

NEW_ASSET="$UPSTREAM_STATE/releases/rabbytesoftware/quiver.core/$CORE_V2/quiver-linux-$(core_arch)"
info "upstream asset: $NEW_ASSET"

# --- assert: core NOTICES, through its own drift check ---------------------

say "Waiting for quiver.core's own version check to notice"
# GET /v0/arrow/:ns runs the version check; version_check_ttl is pinned to 1s
# in this run's config.yaml so repeated polls really do re-check.
found_outdated=0
for _ in $(seq 1 40); do
	arrow_detail "$CORE_ARROW" >/dev/null
	sleep 1
	if [ "$(arrow_field "$CORE_ARROW" '.data.outdated')" = "true" ]; then
		found_outdated=1
		break
	fi
done
[ "$found_outdated" = "1" ] || fail "quiver.core never marked its own arrow outdated after $CORE_V2 was published: $(arrow_detail "$CORE_ARROW" | jq -c '.data | {state,resolved_ref,outdated,available}')"
ok "quiver.core's own arrow reports outdated=true"
assert_eq "$CORE_V2" "$(arrow_field "$CORE_ARROW" '.data.available.ref')" \
	"the release the version check found ahead"
assert_eq "$CORE_V1" "$(arrow_field "$CORE_ARROW" '.data.resolved_ref')" \
	"what the row still has installed"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-outdated.json"

# --- act: run the update ---------------------------------------------------

say "Executing quiver.core's own update lifecycle"
api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW")/update" '{}' 202 >/dev/null
ok "POST /v0/runtime/$CORE_ARROW/update accepted (202)"

# --- assert: the process actually became the new build ---------------------

say "Waiting for the swap"
# The updater stops the old daemon, puts the new binary at the self path and
# starts it on the same socket, so the assertion is a different process
# answering as the new version.
became_v2=0
for _ in $(seq 1 120); do
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
[ "$DAEMON_PID_BEFORE" != "$DAEMON_PID_AFTER" ] || fail "the same pid answers after the swap: the daemon was not replaced"
ok "a new process holds the socket ($DAEMON_PID_BEFORE -> $DAEMON_PID_AFTER)"

# The self path is what a later cold start, or quiver.desktop spawning a fresh
# sidecar, will pick up.
assert_file "$HOME/.quiver/self/quiver" "the swapped-in self-installed binary"
# `quiver version` reports two lines when a daemon is reachable (client and
# daemon), so this asserts on the client line, which is the one that names the
# binary being run.
PROMOTED_VERSION="$("$HOME/.quiver/self/quiver" version | awk '/^client /{print $2}')"
assert_eq "$CORE_V2" "$PROMOTED_VERSION" \
	"the version of the self-installed binary"
assert_eq "$(sha256_of "$BUILD_BIN/quiver-$CORE_V2")" "$(sha256_of "$HOME/.quiver/self/quiver")" \
	"the self-installed binary is byte-identical to the published $CORE_V2 asset"
left="$(ls "$HOME/.quiver/self" | grep -c '^quiver.old-' || true)"
assert_eq "0" "$left" "previous binaries left beside the self path"

# --- assert: the download really happened, over the wire -------------------

say "Proving the new binary came down the real fetch path"
served="$(grep -c "\"event\": \"asset.served\".*\"tag\": \"$CORE_V2\"" "$RESULTS/upstream.log" || true)"
[ "${served:-0}" -ge 1 ] || fail "the upstream fixture never served the $CORE_V2 asset; the update cannot have fetched it"
ok "the upstream fixture served the $CORE_V2 asset $served time(s)"
grep "\"tag\": \"$CORE_V2\"" "$RESULTS/upstream.log" >"$SCENARIO_DIR/upstream-hits.log" || true

# --- assert: the catalog moved on -----------------------------------------

say "Checking the catalog after the restart"
# The relaunched build adopts its own state on boot: the SAME row now has
# $CORE_V2 installed and nothing ahead. No row for either version's name
# exists; GET on one would still answer 200 via the live preview of an
# uncatalogued identity, so the catalog listing is what has to be asserted.
advanced=0
for _ in $(seq 1 30); do
	[ "$(catalogued_refs_resolved "$CORE_NS")" = "$CORE_V2" ] && { advanced=1; break; }
	sleep 1
done
api_body GET /v0/arrow | jq '.' >"$SCENARIO_DIR/catalog-after.json"
arrow_detail "$CORE_ARROW" | jq '.' >"$SCENARIO_DIR/self-arrow-after.json"
[ "$advanced" = "1" ] || fail "quiver.core@stable never recorded $CORE_V2 as installed: $(catalogued_refs_resolved "$CORE_NS")"
assert_eq "stable" "$(catalogued_refs "$CORE_NS")" \
	"the quiver.core rows left in the catalog after the handover"
assert_eq "absent" "$(arrow_field "$CORE_ARROW" '.data.available // "absent"')" \
	"what the version check finds ahead of $CORE_V2"
assert_eq "Quiver Core" "$(arrow_field "$CORE_ARROW" '.data.name')" \
	"the self-arrow's name after the handover"

api_body GET /versions | jq '.' >"$SCENARIO_DIR/versions-after.json"
scenario_end
